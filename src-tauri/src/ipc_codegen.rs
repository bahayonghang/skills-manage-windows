#![cfg(feature = "ipc-codegen")]

use std::borrow::Cow;
use std::collections::HashSet;
use std::io;
use std::path::Path;

use specta::datatype::{DataType, NamedReferenceType, Reference};
use specta::{Format, FormatError, Types};
use specta_serde::{Phase, PhasesFormat};
use specta_typescript::{Error, Exporter, FrameworkExporter, Typescript};
use tauri_specta::{Builder, BuilderConfiguration, LanguageExt};

use crate::commands;

macro_rules! collect_generated_commands {
    ($($name:ident => $($command:ident)::+,)+) => {
        tauri_specta::collect_commands![$($($command)::+),+]
    };
}

#[allow(deprecated)]
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(crate::__skillport_generated_commands!(
        collect_generated_commands
    ))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AdapterContractExporter;

impl LanguageExt for AdapterContractExporter {
    type Error = Error;

    fn export(self, cfg: &BuilderConfiguration, path: &Path) -> Result<(), Self::Error> {
        let cfg = cfg.clone();
        let types = cfg.types.clone();
        let rendered = Exporter::from(Typescript::default())
            .framework_prelude(
                "// Generated from Rust/Serde metadata by `pnpm ipc:codegen`. Do not edit.\n\
                 // This artifact contains contract metadata only and never invokes Tauri.",
            )
            .framework_runtime(move |mut exporter| {
                let contract = render_contract(&cfg, &exporter)?;
                let user_types = exporter.render_types()?;
                Ok(Cow::Owned(format!("{contract}\n{user_types}")))
            })
            .export(&types, AdapterFormat)?;
        let rendered = format!("{}\n", rendered.trim_end_matches(['\r', '\n']));

        std::fs::write(path, rendered).map_err(Error::from)
    }
}

#[derive(Debug, Clone, Copy)]
struct AdapterFormat;

impl Format for AdapterFormat {
    fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, FormatError> {
        PhasesFormat.map_types(types)
    }

    fn map_type(
        &'_ self,
        types: &Types,
        datatype: &DataType,
    ) -> Result<Cow<'_, DataType>, FormatError> {
        PhasesFormat.map_type(types, datatype)
    }
}

fn render_contract(
    cfg: &BuilderConfiguration,
    exporter: &FrameworkExporter<'_>,
) -> Result<String, Error> {
    let mut commands = cfg.commands.iter().collect::<Vec<_>>();
    commands.sort_by(|left, right| left.name().cmp(right.name()));

    let mut seen = HashSet::new();
    let mut map = String::from(
        "type GeneratedIpcCommandSpec<Args, Result> = { args: Args; result: Result };\n\
         const command = <Args, Result>() => ({}) as GeneratedIpcCommandSpec<Args, Result>;\n\n\
         export const GENERATED_IPC_COMMANDS = {\n",
    );
    let mut names = Vec::with_capacity(commands.len());

    for command in commands {
        if !seen.insert(command.name()) {
            return Err(contract_error(format!(
                "duplicate generated command `{}`",
                command.name()
            )));
        }
        names.push(command.name());

        let args = if command.args().is_empty() {
            "undefined".to_string()
        } else {
            let fields = command
                .args()
                .iter()
                .map(|(name, datatype)| {
                    Ok(format!(
                        "{}: {}",
                        lower_camel(name),
                        render_phase(datatype, Phase::Deserialize, exporter)?
                    ))
                })
                .collect::<Result<Vec<_>, Error>>()?;
            format!("{{ {} }}", fields.join("; "))
        };

        let result = command
            .result()
            .ok_or_else(|| contract_error(format!("command `{}` has no result", command.name())))?;
        let (ok, error) = extract_result(result, exporter.types).ok_or_else(|| {
            contract_error(format!(
                "command `{}` must return IpcResult<T>",
                command.name()
            ))
        })?;
        if !is_ipc_error(error, exporter.types) {
            return Err(contract_error(format!(
                "command `{}` has an unexpected error type",
                command.name()
            )));
        }
        let result = render_phase(ok, Phase::Serialize, exporter)?;
        map.push_str(&format!(
            "  {}: command<{}, {}>(),\n",
            command.name(),
            args,
            result
        ));
    }
    map.push_str("} as const;\n\nexport const GENERATED_IPC_COMMAND_NAMES = [\n");
    for name in names {
        map.push_str(&format!("  \"{name}\",\n"));
    }
    map.push_str("] as const;\n");

    if map.contains("unknown") {
        return Err(contract_error(
            "generated IPC contract degraded to `unknown`".to_string(),
        ));
    }
    Ok(map)
}

fn render_phase(
    datatype: &DataType,
    phase: Phase,
    exporter: &FrameworkExporter<'_>,
) -> Result<String, Error> {
    let datatype = specta_serde::select_phase_datatype(datatype, exporter.types, phase);
    match &datatype {
        DataType::Reference(reference) => exporter.reference(reference),
        datatype => exporter.inline(datatype),
    }
}

fn extract_result<'a>(
    datatype: &'a DataType,
    types: &'a Types,
) -> Option<(&'a DataType, &'a DataType)> {
    let DataType::Reference(Reference::Named(reference)) = datatype else {
        return None;
    };
    let named = types.get(reference)?;
    if named.name != "Result" || !matches!(&*named.module_path, "std::result" | "core::result") {
        return None;
    }
    let NamedReferenceType::Reference { generics, .. } = &reference.inner else {
        return None;
    };
    let [(_, ok), (_, error), ..] = generics.as_slice() else {
        return None;
    };
    Some((ok, error))
}

fn is_ipc_error(datatype: &DataType, types: &Types) -> bool {
    let DataType::Reference(Reference::Named(reference)) = datatype else {
        return false;
    };
    types
        .get(reference)
        .is_some_and(|named| named.name == "IpcError")
}

fn lower_camel(name: &str) -> String {
    let mut pieces = name.split('_');
    let mut output = pieces.next().unwrap_or_default().to_string();
    for piece in pieces {
        let mut chars = piece.chars();
        if let Some(first) = chars.next() {
            output.extend(first.to_uppercase());
            output.extend(chars);
        }
    }
    output
}

fn contract_error(message: String) -> Error {
    Error::framework("adapter IPC contract", io::Error::other(message))
}

mod rename_before {
    use crate::ipc_error::IpcResult;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, specta::Type)]
    #[serde(rename = "RenameFixturePayload")]
    pub struct Payload {
        #[serde(rename = "oldName")]
        value: String,
    }

    #[tauri::command]
    #[specta::specta(rename = "rename_fixture")]
    pub fn command_before(request: Payload) -> IpcResult<()> {
        let _ = request;
        Ok(())
    }
}

mod rename_after {
    use crate::ipc_error::IpcResult;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, specta::Type)]
    #[serde(rename = "RenameFixturePayload")]
    pub struct Payload {
        #[serde(rename = "newName")]
        value: String,
    }

    #[tauri::command]
    #[specta::specta(rename = "rename_fixture")]
    pub fn command_after(request: Payload) -> IpcResult<()> {
        let _ = request;
        Ok(())
    }
}

pub fn verify_serde_rename_drift() -> Result<(), Error> {
    let before = export_fixture(tauri_specta::collect_commands![
        rename_before::command_before
    ])?;
    let after = export_fixture(tauri_specta::collect_commands![rename_after::command_after])?;

    if !before.contains("oldName")
        || before.contains("newName")
        || !after.contains("newName")
        || before.as_bytes() == after.as_bytes()
    {
        return Err(contract_error(
            "Serde field rename did not change the checked artifact".to_string(),
        ));
    }
    Ok(())
}

pub fn run_tool(check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let output = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src")
        .join("lib")
        .join("ipc")
        .join("generatedCommandMap.ts");

    if check {
        verify_serde_rename_drift()?;
        let directory = tempfile::tempdir()?;
        let candidate = directory.path().join("generatedCommandMap.ts");
        builder().export(AdapterContractExporter, &candidate)?;
        let expected = std::fs::read(&output)?;
        let actual = std::fs::read(&candidate)?;
        if !actual.ends_with(b"\n") || actual.ends_with(b"\n\n") {
            return Err(contract_error(
                "generated IPC artifact must end with exactly one newline".to_string(),
            )
            .into());
        }
        if expected != actual {
            return Err(format!("{} is stale; run `pnpm ipc:codegen`", output.display()).into());
        }
        println!("[ipc-codegen] checked {}", output.display());
        return Ok(());
    }

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    builder().export(AdapterContractExporter, &output)?;
    println!("[ipc-codegen] generated {}", output.display());
    Ok(())
}

fn export_fixture(commands: tauri_specta::Commands<tauri::Wry>) -> Result<String, Error> {
    let directory = tempfile::tempdir().map_err(Error::from)?;
    let output = directory.path().join("fixture.ts");
    Builder::<tauri::Wry>::new()
        .commands(commands)
        .export(AdapterContractExporter, &output)?;
    std::fs::read_to_string(output).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_counts_and_subset_are_frozen() {
        assert_eq!(crate::ipc_registry::RUNTIME_COMMAND_NAMES.len(), 184);
        assert_eq!(crate::ipc_registry::GENERATED_COMMAND_NAMES.len(), 42);
        for command in crate::ipc_registry::GENERATED_COMMAND_NAMES {
            assert!(crate::ipc_registry::RUNTIME_COMMAND_NAMES.contains(command));
        }
    }

    #[test]
    fn command_argument_names_are_lower_camel_case() {
        assert_eq!(lower_camel("repository_ids"), "repositoryIds");
        assert_eq!(lower_camel("value"), "value");
    }
}

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, FileText, Import, Plus, RefreshCw, Search } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { RuleEditor } from "@/components/rules/RuleEditor";
import { formatBackendError } from "@/lib/backendError";
import { isTauriRuntime } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { isRuleDraftDirty, useRulesStore } from "@/stores/rulesStore";
import { useTargetStore } from "@/stores/targetStore";
import type { RuleTargetStatus } from "@/types/rules";

type Confirmation = { kind: "switch"; name: string } | { kind: "delete" } | { kind: "takeover"; target: RuleTargetStatus } | { kind: "recover"; operationId: string; backupPath: string | null };

export function RulesView() {
  const { t } = useTranslation();
  const rules = useRulesStore();
  const target = useTargetStore((state) => state.activeTarget);
  const [query, setQuery] = useState("");
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [importNames, setImportNames] = useState<string[]>([]);
  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [newBody, setNewBody] = useState("");
  const [showDetail, setShowDetail] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);
  const local = target.kind === "local";
  const draft = rules.selectedName ? rules.drafts[rules.selectedName] : undefined;
  const filtered = rules.snapshot?.rules.filter((rule) => `${rule.name} ${rule.title}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())) ?? [];
  const busy = rules.mutating || rules.saving || Object.values(rules.pendingTools).some(Boolean);

  const run = async (action: () => Promise<void>, success?: () => void) => {
    setDialogError(null);
    rules.clearError();
    try { await action(); success?.(); }
    catch (error) { const message = formatBackendError(error, t); setDialogError(message); toast.error(message); }
  };

  useEffect(() => {
    void useRulesStore.getState().load().catch(() => { });
    setConfirmation(null);
    setImportOpen(false);
    setCreateOpen(false);
    setDialogError(null);
  }, [target.id]);

  const selectRule = (name: string) => {
    if (name === rules.selectedName) { setShowDetail(true); return; }
    setDialogError(null);
    if (isRuleDraftDirty(draft)) setConfirmation({ kind: "switch", name });
    else void run(() => rules.select(name), () => setShowDetail(true));
  };

  const openImport = () => {
    setImportNames([]);
    setImportOpen(true);
    void run(() => rules.previewImport(), () => setImportNames(useRulesStore.getState().importPreview?.entries.filter((entry) => entry.status === "new").map((entry) => entry.name) ?? []));
  };

  const toggleTarget = (toolTarget: RuleTargetStatus) => {
    if (toolTarget.state === "native_equivalent") { setDialogError(null); setConfirmation({ kind: "takeover", target: toolTarget }); }
    else void run(() => rules.setTargetEnabled(toolTarget.tool, toolTarget.state !== "linked"));
  };

  const confirm = () => {
    if (!confirmation) return;
    if (confirmation.kind === "delete") void run(rules.deleteRule, () => setConfirmation(null));
    else if (confirmation.kind === "takeover") void run(() => rules.setTargetEnabled(confirmation.target.tool, true, confirmation.target.fingerprint), () => setConfirmation(null));
    else if (confirmation.kind === "recover") void run(() => rules.recover(confirmation.operationId), () => setConfirmation(null));
  };

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <header className="shrink-0 border-b border-border px-4 py-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex min-w-0 items-center gap-2.5">
            <FileText aria-hidden className="size-5 text-primary" />
            <h1 className="font-heading text-base font-semibold">
              {t("rules.title")}
            </h1>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="ghost" disabled={!local || busy || rules.loading} onClick={() => void run(rules.load)}>
              <RefreshCw aria-hidden />
              {t("common.refresh")}
            </Button>
            <Button variant="outline" disabled={!local || busy} onClick={openImport}>
              <Import aria-hidden />
              {t("rules.importExisting")}
            </Button>
            <Button disabled={!local || busy} onClick={() => { setDialogError(null); setCreateOpen(true); }}>
              <Plus aria-hidden />
              {t("rules.newRule")}
            </Button>
          </div>
        </div>
        <p className="mt-2 break-all font-mono text-ui-meta text-muted-foreground">{local ? rules.snapshot?.rootPath ?? "~/.skillport/rules" : t("rules.localOnly")}
        </p>
        {!isTauriRuntime() && <p className="mt-1 text-xs text-muted-foreground">
          {t("rules.fixtureNotice")}
        </p>}
      </header>
      {rules.errorCode && <div role="alert" className="shrink-0 border-b border-border px-4 py-2 text-sm text-destructive-text">{formatBackendError(`${rules.errorCode}:`, t)} {rules.requiresReload && t("rules.reloadRequired")}
      </div>}
      {local && rules.snapshot?.recoveryOperations.map((operation) => <div key={operation.operationId} className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b border-border bg-warning/5 px-4 py-2">
        <span className="break-all text-xs text-warning">
          {t("rules.recoveryNeeded", { name: operation.name })}
        </span>
        <Button variant="outline" disabled={busy} onClick={() => { setDialogError(null); setConfirmation({ kind: "recover", operationId: operation.operationId, backupPath: operation.backupPath }); }}>
          {t("rules.reviewRecovery")}
        </Button>
      </div>)}
      {!local ? <div className="p-6 text-sm text-muted-foreground">
        {t("rules.localOnlyDetail")}
      </div> : (
        <div className="flex min-h-0 flex-1">
          <aside aria-label={t("rules.listTitle")} className={cn("flex min-h-0 w-full shrink-0 flex-col border-r border-border bg-card min-[720px]:w-44 min-[1100px]:w-56", showDetail && "hidden min-[720px]:flex")}>
            <div className="shrink-0 border-b border-border p-3">
              <label className="relative block">
                <Search aria-hidden className="pointer-events-none absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
                <Input aria-label={t("rules.search")} placeholder={t("rules.search")} value={query} onChange={(event) => setQuery(event.target.value)} className="pl-8" />
              </label>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto">
              {rules.loading && !rules.snapshot ? <p role="status" className="p-4 text-sm text-muted-foreground">
                {t("common.loading")}
              </p> : filtered.length ? filtered.map((rule) => <button key={rule.name} onClick={() => selectRule(rule.name)} aria-label={t("rules.selectRule", { name: rule.name })} aria-pressed={rules.selectedName === rule.name} className={cn("w-full border-b border-border px-3 py-3 text-left outline-none hover:bg-muted/60 focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring", rules.selectedName === rule.name && "bg-primary/10")}>
                <span className="block break-words text-sm font-medium">{rule.title || rule.name}
                </span>
                <span className="mt-1 block break-all font-mono text-ui-meta text-muted-foreground">{rule.name}
                </span>
                <span className="mt-2 flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">{rule.targets.map((item) => <span key={item.tool}>
                  {t(`rules.tools.${item.tool}`)}: {t(`rules.states.${item.state}`)}
                </span>)}
                </span>
                {isRuleDraftDirty(rules.drafts[rule.name]) && <span className="mt-1 block text-xs text-warning">
                  {t("rules.unsaved")}
                </span>}
              </button>) : <p className="p-4 text-sm text-muted-foreground">{query ? t("rules.noResults") : t("rules.empty")}
              </p>}
            </div>
          </aside>
          <main className={cn("flex min-h-0 min-w-0 flex-1 flex-col", !showDetail && "hidden min-[720px]:flex")}>
            <div className="shrink-0 px-4 pt-2 min-[720px]:hidden">
              <Button variant="ghost" onClick={() => setShowDetail(false)}>
                <ArrowLeft aria-hidden />
                {t("rules.backToList")}
              </Button>
            </div>
            {rules.detailLoading ? <p role="status" className="p-5 text-sm text-muted-foreground">
              {t("common.loading")}
            </p> : rules.detail ? <RuleEditor key={rules.detail.name} detail={rules.detail} draft={draft} saving={rules.saving} disabled={rules.mutating} pendingTools={rules.pendingTools} onEdit={rules.editBody} onSave={() => void run(rules.save)} onDelete={() => { setDialogError(null); setConfirmation({ kind: "delete" }); }} onToggle={toggleTarget} /> : <div className="p-5 text-sm text-muted-foreground">{rules.errorCode ? t("rules.retryHint") : t("rules.selectHint")}
            </div>}
          </main>
        </div>
      )}

      <Dialog open={createOpen} onOpenChange={(open) => { if (!busy) { setCreateOpen(open); setDialogError(null); } }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {t("rules.newRule")}
            </DialogTitle>
            <DialogDescription>
              {t("rules.newRuleHint")}
            </DialogDescription>
          </DialogHeader>
          <DialogBody className="space-y-3">
            <label className="block space-y-1.5">
              <span>
                {t("rules.fileName")}
              </span>
              <Input value={newName} onChange={(event) => setNewName(event.target.value)} placeholder={t("rules.fileNamePlaceholder")} />
            </label>
            <label className="block space-y-1.5">
              <span>
                {t("rules.description")}
              </span>
              <Input value={newDescription} onChange={(event) => setNewDescription(event.target.value)} />
            </label>
            <label className="block space-y-1.5">
              <span>
                {t("rules.bodyLabel")}
              </span>
              <Textarea className="min-h-40 font-mono text-sm" value={newBody} onChange={(event) => setNewBody(event.target.value)} />
            </label>{dialogError && <p role="alert" className="text-sm text-destructive-text">{dialogError}
            </p>}
          </DialogBody>
          <DialogFooter>
            <Button variant="outline" disabled={busy} onClick={() => setCreateOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button disabled={busy || !newName.trim()} onClick={() => void run(() => rules.createRule(newName.trim(), newDescription, newBody), () => { setCreateOpen(false); setNewName(""); setNewDescription(""); setNewBody(""); setShowDetail(true); })}>
              {t("common.create")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={importOpen} onOpenChange={(open) => { if (!busy) { setImportOpen(open); setDialogError(null); } }}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>
              {t("rules.importExisting")}
            </DialogTitle>
            <DialogDescription>
              {t("rules.importHint")}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>{rules.mutating && !rules.importPreview ? <p role="status">
            {t("common.loading")}
          </p> : rules.importPreview?.entries.length ? <div className="divide-y divide-border">{rules.importPreview.entries.map((entry) => {
            const result = rules.importResult?.entries.find((item) => item.name === entry.name); const eligible = entry.status === "new" || entry.status === "same"; return <label key={entry.name} className="flex items-start gap-3 py-3">
              <input type="checkbox" className="mt-1 size-4 accent-primary" disabled={!eligible || busy || !!rules.importResult} checked={importNames.includes(entry.name)} onChange={(event) => setImportNames((names) => event.target.checked ? [...names, entry.name] : names.filter((name) => name !== entry.name))} />
              <span className="min-w-0 flex-1">
                <span className="block break-all font-mono text-sm">{entry.name}
                </span>
                <span className="mt-1 block text-xs text-muted-foreground">
                  {t(`rules.importStates.${result?.status ?? entry.status}`)}
                </span>
                <span className="mt-1 block text-ui-meta text-muted-foreground">{entry.sourceFingerprints.map((source) => t(`rules.tools.${source.tool}`)).join(" · ")}
                </span>{(result?.errorCode ?? entry.errorCode) && <span className="mt-1 block text-xs text-destructive-text">{formatBackendError(`${result?.errorCode ?? entry.errorCode}:`, t)}
                </span>}
              </span>
            </label>;
          })}
          </div> : <p>
            {t("rules.noImportSources")}
          </p>}{dialogError && <p role="alert" className="mt-3 text-sm text-destructive-text">{dialogError}
          </p>}
          </DialogBody>
          <DialogFooter>
            <Button variant="outline" disabled={busy} onClick={() => setImportOpen(false)}>
              {t("common.close")}
            </Button>
            <Button disabled={busy || !importNames.length || !!rules.importResult} onClick={() => void run(() => rules.importRules(importNames))}>{busy ? t("rules.importing") : t("rules.importSelected", { count: importNames.length })}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!confirmation} onOpenChange={(open) => { if (!open && !busy) { setConfirmation(null); setDialogError(null); } }}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{confirmation?.kind === "switch" ? t("rules.unsavedTitle") : confirmation?.kind === "takeover" ? t("rules.takeOverTitle") : confirmation?.kind === "recover" ? t("rules.recoveryTitle") : t("rules.deleteTitle")}
            </DialogTitle>
            <DialogDescription>{confirmation?.kind === "switch" ? t("rules.unsavedHint") : confirmation?.kind === "takeover" ? t("rules.takeOverHint") : confirmation?.kind === "recover" ? t("rules.recoveryHint") : t("rules.deleteHint", { name: rules.selectedName ?? "" })}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>{confirmation?.kind === "takeover" && <div className="space-y-2">
            <p className="break-all font-mono text-ui-meta">{confirmation.target.path}
            </p>
            <p className="text-xs text-muted-foreground">
              {t("rules.backupLocation")}
            </p>
            <p className="break-all font-mono text-ui-meta">{rules.snapshot?.rootPath}/.backups</p>
          </div>}{confirmation?.kind === "delete" && <ul className="space-y-2">{rules.detail?.targets.map((item) => <li key={item.tool} className="text-xs">
            {t(`rules.tools.${item.tool}`)}: {t(`rules.states.${item.state}`)}<p className="break-all font-mono text-ui-meta text-muted-foreground">{item.path}
            </p>
          </li>)}
          </ul>}{confirmation?.kind === "recover" && confirmation.backupPath && <p className="break-all font-mono text-ui-meta">{confirmation.backupPath}
          </p>}{dialogError && <p role="alert" className="mt-2 text-sm text-destructive-text">{dialogError}
          </p>}
          </DialogBody>
          <DialogFooter>
            <Button variant="outline" disabled={busy} onClick={() => setConfirmation(null)}>
              {t("common.cancel")}
            </Button>{confirmation?.kind === "switch" ? <>
              <Button variant="outline" disabled={busy} onClick={() => { const name = confirmation.name; rules.discardDraft(); void run(() => rules.select(name), () => { setConfirmation(null); setShowDetail(true); }); }}>
                {t("rules.discard")}
              </Button>
              <Button disabled={busy} onClick={() => { const name = confirmation.name; void run(async () => { await rules.save(); await rules.select(name); }, () => { setConfirmation(null); setShowDetail(true); }); }}>
                {t("rules.saveAndSwitch")}
              </Button>
            </> : <Button variant={confirmation?.kind === "delete" ? "destructive" : "default"} disabled={busy || !local} onClick={confirm}>{confirmation?.kind === "takeover" ? t("rules.takeOver") : confirmation?.kind === "recover" ? t("rules.recover") : t("common.delete")}
            </Button>}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

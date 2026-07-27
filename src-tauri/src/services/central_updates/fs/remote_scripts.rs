pub(super) const REMOTE_STAGE_UPDATE: &str = r#"
set -eu
operation_id=$1
staging=$2
marker=$3
extract_root=$4
[ ! -e "$staging" ] && [ ! -L "$staging" ] || exit 71
[ ! -e "$marker" ] || exit 72
rm -rf -- "$extract_root"
mkdir -p -- "$extract_root" || exit 73
cleanup() { rm -rf -- "$extract_root" 2>/dev/null || true; }
trap cleanup EXIT HUP INT TERM
printf '%s' "$operation_id" > "$marker" || exit 77
tar -xzf - -C "$extract_root" || exit 74
[ -d "$extract_root/0000" ] || exit 75
mv -- "$extract_root/0000" "$staging" || exit 76
printf 'STAGED\n'
"#;

pub(super) const REMOTE_BATCH_STAGE_UPDATE: &str = r#"
set -u
batch_root=$1
manifest="$batch_root/.skillport-operation-manifest.tsv"

cleanup() { rm -rf -- "$batch_root" 2>/dev/null || true; }
trap cleanup EXIT HUP INT TERM
rm -rf -- "$batch_root"
mkdir -p -- "$batch_root" || exit 70
tar -xzf - -C "$batch_root" || exit 71
[ -f "$manifest" ] || exit 72

tab=$(printf '\t')
while IFS="$tab" read -r archive_key skill_id operation_id staging marker; do
  [ -n "$archive_key" ] || continue
  case "$archive_key" in *[!0-9]*) printf 'ERR\t%s\tinvalid_manifest\n' "$skill_id"; continue ;; esac
  case "$skill_id" in ''|*[!A-Za-z0-9._-]*) printf 'ERR\t%s\tinvalid_manifest\n' "$skill_id"; continue ;; esac
  case "$operation_id" in ''|*[!A-Za-z0-9._-]*) printf 'ERR\t%s\tinvalid_manifest\n' "$skill_id"; continue ;; esac
  source_dir="$batch_root/$archive_key"
  if [ ! -d "$source_dir" ] || [ -e "$staging" ] || [ -L "$staging" ] || [ -e "$marker" ]; then
    printf 'ERR\t%s\tstage_collision\n' "$skill_id"
    continue
  fi
  if ! printf '%s' "$operation_id" > "$marker"; then
    printf 'ERR\t%s\tmarker_failed\n' "$skill_id"
    continue
  fi
  if mv -- "$source_dir" "$staging"; then
    printf 'OK\t%s\n' "$skill_id"
  else
    if rm -f -- "$marker"; then
      printf 'ERR\t%s\tstaging_failed\n' "$skill_id"
    else
      printf 'ERR\t%s\tmarker_cleanup_failed\n' "$skill_id"
    fi
  fi
done < "$manifest"
"#;

pub(super) const REMOTE_SWAP_UPDATE: &str = r#"
set -eu
operation_id=$1
target=$2
staging=$3
backup=$4
marker=$5
had_target=$6
[ -f "$marker" ] && [ "$(cat "$marker")" = "$operation_id" ] || exit 81
[ -d "$staging" ] || exit 82
[ ! -e "$backup" ] && [ ! -L "$backup" ] || exit 83
if [ "$had_target" = 1 ]; then
  ([ -e "$target" ] || [ -L "$target" ]) || exit 84
  mv -- "$target" "$backup" || exit 85
else
  [ ! -e "$target" ] && [ ! -L "$target" ] || exit 84
fi
if mv -- "$staging" "$target"; then
  printf 'SWAPPED\n'
else
  if [ "$had_target" = 1 ] && ([ -e "$backup" ] || [ -L "$backup" ]); then
    mv -- "$backup" "$target" || exit 87
  fi
  exit 86
fi
"#;

pub(super) const REMOTE_ROLLBACK_UPDATE: &str = r#"
set -eu
operation_id=$1
target=$2
staging=$3
backup=$4
marker=$5
had_target=$6
if [ ! -e "$marker" ]; then
  [ ! -e "$staging" ] && [ ! -L "$staging" ] || exit 91
  [ ! -e "$backup" ] && [ ! -L "$backup" ] || exit 91
  if [ "$had_target" = 1 ]; then
    ([ -e "$target" ] || [ -L "$target" ]) || exit 91
  else
    [ ! -e "$target" ] && [ ! -L "$target" ] || exit 91
  fi
  printf 'ROLLED_BACK\n'
  exit 0
fi
[ -f "$marker" ] && [ "$(cat "$marker")" = "$operation_id" ] || exit 91
if [ -e "$backup" ] || [ -L "$backup" ]; then
  rm -rf -- "$target" || exit 92
  mv -- "$backup" "$target" || exit 93
elif [ "$had_target" = 0 ]; then
  rm -rf -- "$target" || exit 92
fi
rm -rf -- "$staging" || exit 94
rm -f -- "$marker" || exit 95
printf 'ROLLED_BACK\n'
"#;

pub(super) const REMOTE_FINALIZE_UPDATE: &str = r#"
set -eu
operation_id=$1
target=$2
staging=$3
backup=$4
marker=$5
if [ ! -e "$marker" ]; then
  [ -d "$target" ] || exit 102
  [ ! -e "$backup" ] && [ ! -L "$backup" ] || exit 101
  [ ! -e "$staging" ] && [ ! -L "$staging" ] || exit 101
  printf 'FINALIZED\n'
  exit 0
fi
[ -f "$marker" ] && [ "$(cat "$marker")" = "$operation_id" ] || exit 101
[ -d "$target" ] || exit 102
rm -rf -- "$backup" "$staging" || exit 103
rm -f -- "$marker" || exit 104
printf 'FINALIZED\n'
"#;

/// Apply a tar.gz containing `.skillport-manifest.tsv` plus one directory per
/// manifest key. Archive extraction completes before any canonical target is
/// touched; each subsequent swap reports an independent outcome.
#[cfg(test)]
pub(super) const REMOTE_CENTRAL_BATCH_UPDATE_SCRIPT: &str = r#"
set -u

batch_root=$1
manifest="$batch_root/.skillport-manifest.tsv"

cleanup() {
  rm -rf -- "$batch_root" 2>/dev/null || true
}
trap cleanup EXIT HUP INT TERM

rm -rf -- "$batch_root"
mkdir -p -- "$batch_root" || exit 10
tar -xzf - -C "$batch_root" || exit 11
[ -f "$manifest" ] || exit 12

tab=$(printf '\t')
while IFS="$tab" read -r archive_key skill_id target_dir; do
  [ -n "$archive_key" ] || continue
  case "$archive_key" in *[!0-9]*) printf 'ERR\t%s\tinvalid_manifest\n' "$skill_id"; continue ;; esac
  case "$skill_id" in ''|*[!A-Za-z0-9._-]*) printf 'ERR\t%s\tinvalid_manifest\n' "$skill_id"; continue ;; esac
  source_dir="$batch_root/$archive_key"
  parent_dir=${target_dir%/*}
  staging_dir="$parent_dir/.skillport-update-$skill_id-$archive_key-$$"
  backup_dir="$parent_dir/.skillport-backup-$skill_id-$archive_key-$$"

  if [ "$parent_dir" = "$target_dir" ] || [ ! -d "$source_dir" ]; then
    printf 'ERR\t%s\tinvalid_manifest\n' "$skill_id"
    continue
  fi
  if ! mkdir -p -- "$parent_dir" || ! rm -rf -- "$staging_dir" "$backup_dir" || ! mv "$source_dir" "$staging_dir"; then
    printf 'ERR\t%s\tstaging_failed\n' "$skill_id"
    continue
  fi

  had_target=0
  if [ -e "$target_dir" ]; then
    if mv "$target_dir" "$backup_dir"; then
      had_target=1
    else
      rm -rf -- "$staging_dir" 2>/dev/null || true
      printf 'ERR\t%s\tbackup_failed\n' "$skill_id"
      continue
    fi
  fi

  if mv "$staging_dir" "$target_dir"; then
    rm -rf -- "$backup_dir" 2>/dev/null || true
    printf 'OK\t%s\n' "$skill_id"
  else
    if [ "$had_target" -eq 1 ] && [ -e "$backup_dir" ]; then
      mv "$backup_dir" "$target_dir" 2>/dev/null || true
    fi
    rm -rf -- "$staging_dir" 2>/dev/null || true
    printf 'ERR\t%s\tswap_failed\n' "$skill_id"
  fi
done < "$manifest"
"#;

pub(super) const REMOTE_HASH_UNSUPPORTED_EXIT_CODE: &str = "86";
pub(super) const REMOTE_HASH_SCRIPT: &str = r#"
set -eu

if command -v sha256sum >/dev/null 2>&1; then
  hash_cmd='sha256sum'
elif command -v shasum >/dev/null 2>&1; then
  hash_cmd='shasum'
elif command -v openssl >/dev/null 2>&1; then
  hash_cmd='openssl'
else
  exit 86
fi

for root in "$@"; do
  if [ ! -d "$root" ]; then
    printf 'ROOT\t%s\n' "$root"
    printf 'END\t%s\n' "$root"
    continue
  fi
  printf 'ROOT\t%s\n' "$root"
  (cd "$root" && find . -type f -exec sh -c '
    hash_cmd=$1
    shift
    for path do
      case "$hash_cmd" in
        sha256sum) digest=$(sha256sum "$path") ;;
        shasum) digest=$(shasum -a 256 "$path") ;;
        openssl) digest=$(openssl dgst -sha256 -r "$path") ;;
      esac
      set -- $digest
      digest=$1
      rel=${path#./}
      printf "%s\t%s\n" "$digest" "$rel"
    done
  ' sh "$hash_cmd" {} + | LC_ALL=C sort)
  printf 'END\t%s\n' "$root"
done
"#;

pub(super) const REMOTE_BATCH_REFRESH_COPY_SCRIPT: &str = r#"
set -u

while [ "$#" -gt 0 ]; do
  if [ "$#" -lt 3 ]; then exit 20; fi
  skill_id=$1
  source_dir=$2
  target=$3
  shift 3

  case "$target" in
    */"$skill_id"|"$skill_id") ;;
    *)
      printf 'ERR\t%s\tinvalid_target\n' "$skill_id"
      continue
      ;;
  esac

  if rm -rf -- "$target" && mkdir -p -- "$target" && cp -R "$source_dir"/. "$target"/; then
    printf 'OK\t%s\n' "$skill_id"
  else
    printf 'ERR\t%s\tcopy_failed\n' "$skill_id"
  fi
done
"#;

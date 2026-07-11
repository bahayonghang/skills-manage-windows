/// Apply a tar.gz containing `.skillport-manifest.tsv` plus one directory per
/// manifest key. Archive extraction completes before any canonical target is
/// touched; each subsequent swap reports an independent outcome.
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

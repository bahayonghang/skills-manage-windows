/// Atomically replace a remote skill directory by extracting a tar.gz stream
/// into staging, then swapping staging with the canonical directory.
pub(super) const REMOTE_CENTRAL_UPDATE_SCRIPT: &str = r#"
set -eu

target_dir=$1
parent_dir=$2
staging_dir=$3
backup_dir=$4

mkdir -p -- "$parent_dir"
rm -rf -- "$staging_dir"
mkdir -p -- "$staging_dir"
tar -xzf - -C "$staging_dir"

if [ -e "$target_dir" ]; then
  mv "$target_dir" "$backup_dir"
fi

if mv "$staging_dir" "$target_dir"; then
  rm -rf -- "$backup_dir" 2>/dev/null || true
else
  rc=$?
  if [ -e "$backup_dir" ]; then mv "$backup_dir" "$target_dir"; fi
  exit "$rc"
fi
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

pub(super) const REMOTE_REFRESH_COPY_SCRIPT: &str = r#"
set -eu

source_dir=$1
target=$2
skill_id=$3

case "$target" in
  */"$skill_id"|"$skill_id") ;;
  *)
    printf '%s\n' "Refusing to refresh copy install outside expected skill directory '$target'." >&2
    exit 2
    ;;
esac

rm -rf -- "$target"
mkdir -p -- "$target"
cp -R "$source_dir"/. "$target"/
"#;

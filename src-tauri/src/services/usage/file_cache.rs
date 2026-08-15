//! 增量扫描的按文件缓存 —— `skill_call_file_cache` 表的内存形态。
//!
//! provider 在一次 collect 中用它做指纹 diff：
//! - [`ProviderFileCache::lookup`]：文件 (mtime_ms, size) 未变 → 直接返回
//!   缓存的 calls（零磁盘 IO）；
//! - [`ProviderFileCache::record`]：新文件或指纹变化 → 登记本次解析结果，
//!   扫描结束后由编排器批量 upsert；
//! - [`ProviderFileCache::vanished_paths`]：DB 里有记录但盘上已消失的文件，
//!   扫描结束后删除对应缓存行。
//!
//! 合并语义：provider 各自负责把「缓存命中 + 新解析」的 per-file calls 按
//! 与全量扫描相同的顺序、相同的 dedup 规则合并，保证增量结果与全量结果
//! 完全一致（等价性由测试锁定）。

use std::collections::{HashMap, HashSet};

use super::SkillCall;
use crate::db::SkillCallFileCacheRow;

/// 从元数据提取 `(mtime_ms, size)` 指纹。读取失败或时间早于 epoch
/// 时返回 `(0, 0)` —— 与真实指纹相撞的概率≈0，等效于缓存未命中（多解析
/// 一次，不会出错）。错误类型泛化以同时兼容 `std::io::Error`（`std::fs`）
/// 和 `walkdir::Error`（目录遍历）。
pub fn fingerprint_from_metadata<E>(metadata: Result<std::fs::Metadata, E>) -> (i64, i64) {
    match metadata {
        Ok(meta) => {
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as i64)
                .unwrap_or(0);
            (mtime_ms, meta.len() as i64)
        }
        Err(_) => (0, 0),
    }
}

/// 一个文件的缓存内容：指纹 + 解析出的 calls。
#[derive(Debug, Clone)]
pub struct CachedFileCalls {
    pub mtime_ms: i64,
    pub size: i64,
    pub calls: Vec<SkillCall>,
}

/// 待 upsert 的新解析结果。
#[derive(Debug, Clone)]
pub struct CachedFileUpsert {
    pub file_path: String,
    pub mtime_ms: i64,
    pub size: i64,
    pub calls: Vec<SkillCall>,
}

/// 单个 provider 在一次扫描期间的缓存句柄。无状态 default = 空缓存，
/// 即「全量扫描且不落缓存」。
#[derive(Debug, Default)]
pub struct ProviderFileCache {
    /// 扫描开始时从 DB 载入的既有指纹与 calls。
    existing: HashMap<String, CachedFileCalls>,
    /// 本次扫描实际见到的全部文件路径（缓存命中与新解析都算）。
    seen_paths: HashSet<String>,
    /// 本次新解析/指纹变化的文件，扫描结束后 upsert 落库。
    upserts: Vec<CachedFileUpsert>,
}

impl ProviderFileCache {
    /// 从 DB 行构建。`calls_json` 反序列化失败的行按未命中处理——对应文件
    /// 会被当作变化文件重新解析并覆盖，缓存自愈。
    pub fn from_rows(rows: Vec<SkillCallFileCacheRow>) -> Self {
        let existing = rows
            .into_iter()
            .filter_map(|row| {
                let calls = serde_json::from_str(&row.calls_json).ok()?;
                Some((
                    row.file_path,
                    CachedFileCalls {
                        mtime_ms: row.mtime_ms,
                        size: row.size,
                        calls,
                    },
                ))
            })
            .collect();
        Self {
            existing,
            seen_paths: HashSet::new(),
            upserts: Vec::new(),
        }
    }

    /// 指纹未变 → 返回缓存 calls 的克隆；未知文件或指纹变化 → None。
    /// 无论命中与否，path 都会计入 seen（用于 vanished 计算）。
    pub fn lookup(&mut self, path: &str, mtime_ms: i64, size: i64) -> Option<Vec<SkillCall>> {
        self.seen_paths.insert(path.to_string());
        let entry = self
            .existing
            .get(path)
            .filter(|entry| entry.mtime_ms == mtime_ms && entry.size == size)?;
        Some(entry.calls.clone())
    }

    /// 登记一个新解析的文件（内容归调用方所有，缓存克隆一份）。
    pub fn record(&mut self, path: String, mtime_ms: i64, size: i64, calls: Vec<SkillCall>) {
        self.seen_paths.insert(path.clone());
        self.upserts.push(CachedFileUpsert {
            file_path: path,
            mtime_ms,
            size,
            calls,
        });
    }

    /// DB 中有记录但本次扫描未见的文件路径 —— 缓存行待删。
    pub fn vanished_paths(&self) -> Vec<String> {
        self.existing
            .keys()
            .filter(|path| !self.seen_paths.contains(*path))
            .cloned()
            .collect()
    }

    /// 本次扫描产生的待 upsert 行。
    pub fn upserts(&self) -> &[CachedFileUpsert] {
        &self.upserts
    }

    /// 既有指纹与本次新解析的合并视图（同 path 时新解析覆盖旧指纹）。
    /// 供测试把一轮扫描的完整结果「持久化→重载」，等价于 DB round-trip
    /// （`upserts()` 只含本轮变化，稳态下为空，不足以重建整份缓存）。
    #[cfg(test)]
    pub fn snapshot_upserts(&self) -> Vec<CachedFileUpsert> {
        let mut merged: HashMap<String, CachedFileUpsert> = self
            .existing
            .iter()
            .map(|(path, entry)| {
                (
                    path.clone(),
                    CachedFileUpsert {
                        file_path: path.clone(),
                        mtime_ms: entry.mtime_ms,
                        size: entry.size,
                        calls: entry.calls.clone(),
                    },
                )
            })
            .collect();
        for upsert in &self.upserts {
            merged.insert(upsert.file_path.clone(), upsert.clone());
        }
        merged.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(skill: &str) -> SkillCall {
        SkillCall {
            skill: skill.to_string(),
            timestamp_ms: 1,
            project: "/p".to_string(),
            session_id: "s".to_string(),
            source: "Codex CLI".to_string(),
        }
    }

    fn row(path: &str, mtime_ms: i64, size: i64, skills: &[&str]) -> SkillCallFileCacheRow {
        let calls: Vec<SkillCall> = skills.iter().map(|s| call(s)).collect();
        SkillCallFileCacheRow {
            file_path: path.to_string(),
            mtime_ms,
            size,
            calls_json: serde_json::to_string(&calls).unwrap(),
            scanned_at_ms: 0,
        }
    }

    #[test]
    fn fingerprint_from_metadata_reports_mtime_and_size() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("f.jsonl");
        std::fs::write(&path, "hello").unwrap();
        let (mtime_ms, size) = fingerprint_from_metadata(std::fs::metadata(&path));
        assert!(mtime_ms > 0);
        assert_eq!(size, 5);

        assert_eq!(
            fingerprint_from_metadata(std::fs::metadata(dir.path().join("missing"))),
            (0, 0)
        );
    }

    #[test]
    fn lookup_hits_only_on_exact_fingerprint() {
        let mut cache = ProviderFileCache::from_rows(vec![row("/a.jsonl", 100, 10, &["review"])]);
        assert_eq!(cache.lookup("/a.jsonl", 100, 10).map(|c| c.len()), Some(1));
        assert!(cache.lookup("/a.jsonl", 101, 10).is_none(), "mtime drift");
        assert!(cache.lookup("/a.jsonl", 100, 11).is_none(), "size drift");
        assert!(cache.lookup("/b.jsonl", 100, 10).is_none(), "unknown path");
    }

    #[test]
    fn vanished_and_upserts_track_scan_observations() {
        let mut cache = ProviderFileCache::from_rows(vec![
            row("/kept.jsonl", 1, 1, &[]),
            row("/gone.jsonl", 1, 1, &[]),
        ]);
        assert!(cache.lookup("/kept.jsonl", 1, 1).is_some());
        cache.record("/new.jsonl".to_string(), 2, 2, vec![call("facts")]);

        let mut vanished = cache.vanished_paths();
        vanished.sort();
        assert_eq!(vanished, vec!["/gone.jsonl".to_string()]);
        assert_eq!(cache.upserts().len(), 1);
        assert_eq!(cache.upserts()[0].file_path, "/new.jsonl");
        assert_eq!(cache.upserts()[0].calls[0].skill, "facts");
    }

    #[test]
    fn corrupt_calls_json_self_heals_as_cache_miss() {
        let bad = SkillCallFileCacheRow {
            file_path: "/bad.jsonl".to_string(),
            mtime_ms: 1,
            size: 1,
            calls_json: "{not json".to_string(),
            scanned_at_ms: 0,
        };
        let mut cache = ProviderFileCache::from_rows(vec![bad]);
        assert!(cache.lookup("/bad.jsonl", 1, 1).is_none());
    }
}

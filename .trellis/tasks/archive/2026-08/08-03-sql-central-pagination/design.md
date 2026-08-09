# Design: SQL-backed Central pagination

## 1. Request normalization

Service先把DTO转换为内部typed filter：

```rust
struct CentralPageFilter {
    query: Option<String>,
    sources: Vec<String>,
    include_unassigned: bool,
    tags: Vec<String>,
    include_uncategorized: bool,
    install: InstallFilter,
    sort: SortField,
    descending: bool,
    limit: i64,
    offset: i64,
}
```

trim、去空、去`all`、dedupe和100值上限在SQL前完成。动态SQL只根据enum选择固定片段，值全部bind。

## 2. SQL shape

使用两个查询而不是 window count：count在offset超过尾部时仍必须返回真实total。

```text
SELECT COUNT(*) FROM skills s WHERE <predicates>;

SELECT s.* FROM skills s
WHERE <same predicates>
ORDER BY <whitelisted expression>, lower(s.name), s.id
LIMIT ? OFFSET ?;
```

关联filter使用correlated `EXISTS`/`NOT EXISTS`，避免join造成duplicate rows和`DISTINCT`排序成本：

- source：member/repository EXISTS；unassigned为member缺失或repo unknown。
- tags：matching tag EXISTS；uncategorized为不存在任何非-system-uncategorized tag。
- install：direct installation EXISTS，或预先计算的`has_shared_root_agent`让全部Central rows视为linked。

Count/page predicates由同一个builder函数生成，防止total与items drift。

## 3. Text and sort semantics

- Query标准化为 `to_ascii_lowercase`；SQLite `lower`对ASCII保持当前行为。
- 使用`instr`而非`LIKE`，所以 `%`/`_`是普通字符。
- name key：`lower(name)`；time key：`coalesce(fs_*_at, scanned_at)`。
- stable tie：time -> lower(name) -> name -> id；name -> lower(name) -> name -> id。desc只反转主排序方向还是整个tuple必须与reference明确一致；选择整体方向并用tests pin住。

## 4. Timestamp authority

当前 page会在缺失persisted cache时stat filesystem，然后用该值排序。这与SQL page不兼容，也让async hot path I/O随N增长。

决策：paged list只使用persisted cache，null回退`scanned_at`，展示同一值。Scanner/refresh负责更新cache。Detail/unpaged API可保留现有best-effort metadata fallback。本任务更新函数doc和tests，明确这是列表快照语义而非实时filesystem query。

## 5. Page enrichment

Repository返回已分页的`Vec<Skill>`。现有`skills_with_links_from_rows`改为：

1. 获取agents/shared-root IDs一次。
2. 对page IDs批量加载installations、repository assignments、tags。
3. 合成`SkillWithLinks`，时间用persisted helper。

limit最大500，因此每个dynamic IN最多500 binds。若未来page limit改变，helper仍应按统一SQLite bind budget chunk，而不是假设无限。

## 6. Index strategy

先捕获现有plan。当前已有`idx_skills_is_central`/`idx_skills_is_central_name`以及relation PK/index。Contains search预期可能scan Central subset；不要为无法使用B-tree的substring虚构index收益。

若 expression order导致大fixture明显使用temp sort，可评估 `is_central + expression + tie` index。只有plan和benchmark都证明收益才添加versioned migration；每个新增index写明服务的query。

## 7. Testing strategy

- 保留当前pure filter/sort evaluator为 `cfg(test)` reference，补stable tie/persisted timestamp规则。
- 使用pairwise组合加关键全组合，不做随机不确定测试。
- 5k fixture在一个transaction内批量seed，避免test setup本身成为瓶颈。
- Structural hook记录传给enrichment的IDs数量；不要靠最终items长度间接证明。

## 8. Rollback

先新增repository query和shadow equivalence test，旧page path仍为authority；证据一致后切换。Rollback只切回旧route，不删除任何index/data。若timestamp语义未获评审，不开始实施。

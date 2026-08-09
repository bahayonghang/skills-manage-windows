use std::time::Instant;

use super::pagination_tests::{
    ids, percentile, seed_large_pagination_fixture, LARGE_FIXTURE_SKILLS,
};
use super::query::{get_central_skills_page_impl, get_central_skills_page_reference_impl};
use super::types::CentralSkillsPageRequest;
use crate::test_support::mem_pool;

#[tokio::test]
#[ignore = "release-mode evidence benchmark; not a CI timing gate"]
async fn benchmark_central_pagination_large_fixture() {
    let pool = mem_pool().await;
    seed_large_pagination_fixture(&pool).await;
    let request = CentralSkillsPageRequest {
        sort: Some("updatedAt:desc".to_string()),
        limit: Some(25),
        offset: Some(2_500),
        ..Default::default()
    };

    for _ in 0..3 {
        let reference = get_central_skills_page_reference_impl(&pool, request.clone())
            .await
            .expect("warm reference page");
        let sql = get_central_skills_page_impl(&pool, request.clone())
            .await
            .expect("warm SQL page");
        assert_eq!(ids(&sql), ids(&reference));
        assert_eq!(sql.total, LARGE_FIXTURE_SKILLS);
        assert_eq!(sql.items.len(), 25);
    }

    let mut reference_samples = Vec::with_capacity(12);
    for _ in 0..12 {
        let started = Instant::now();
        let page = get_central_skills_page_reference_impl(&pool, request.clone())
            .await
            .expect("benchmark reference page");
        reference_samples.push(started.elapsed());
        assert_eq!(page.total, LARGE_FIXTURE_SKILLS);
        assert_eq!(page.items.len(), 25);
    }

    let mut sql_samples = Vec::with_capacity(12);
    for _ in 0..12 {
        let started = Instant::now();
        let page = get_central_skills_page_impl(&pool, request.clone())
            .await
            .expect("benchmark SQL page");
        sql_samples.push(started.elapsed());
        assert_eq!(page.total, LARGE_FIXTURE_SKILLS);
        assert_eq!(page.items.len(), 25);
    }

    let reference_p50 = percentile(&mut reference_samples.clone(), 50, 100);
    let reference_p95 = percentile(&mut reference_samples, 95, 100);
    let sql_p50 = percentile(&mut sql_samples.clone(), 50, 100);
    let sql_p95 = percentile(&mut sql_samples, 95, 100);
    eprintln!(
        "central-pagination strategy=reference fixture_rows={LARGE_FIXTURE_SKILLS} \
         page_rows=25 enriched_rows={LARGE_FIXTURE_SKILLS} measured_runs=12 \
         p50_us={} p95_us={}",
        reference_p50.as_micros(),
        reference_p95.as_micros()
    );
    eprintln!(
        "central-pagination strategy=sql fixture_rows={LARGE_FIXTURE_SKILLS} \
         page_rows=25 enriched_rows=25 measured_runs=12 p50_us={} p95_us={}",
        sql_p50.as_micros(),
        sql_p95.as_micros()
    );
}

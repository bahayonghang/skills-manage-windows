import { useParams, useNavigate, useLocation, Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { SkillDetailView } from "@/components/skill/SkillDetailView";
import { saveScrollPosition } from "@/lib/scrollRestoration";

/**
 * Shape of the optional `location.state.from` passed by a list page when it
 * navigates to `/skill/:skillId`. Used to render a two-segment breadcrumb
 * in the `PageHeader`. When absent, the breadcrumb collapses to a single
 * segment containing only the skill name / id.
 */
export interface SkillDetailPageFromState {
  pageLabel: string;
  route: string;
}

/**
 * PageHeader — thin row above the shared `SkillDetailView`.
 *
 * Renders `[← Back]` plus a breadcrumb derived from `location.state.from`.
 * The back button always invokes `navigate(-1)` (browser history back); the
 * first breadcrumb segment, when present, is a link to the originating
 * route so users can jump directly to the prior list even if the history
 * stack is missing that entry.
 */
function PageHeader({
  from,
  skillLabel,
  onBack,
}: {
  from: SkillDetailPageFromState | undefined;
  skillLabel: string;
  onBack: () => void;
}) {
  const { t } = useTranslation();

  return (
    <div className="border-b border-border px-6 py-2 flex items-center gap-3 shrink-0">
      <button
        onClick={onBack}
        className="p-1.5 rounded-md hover:bg-muted transition-colors text-muted-foreground hover:text-foreground"
        aria-label={t("detail.goBack")}
      >
        <ArrowLeft className="size-4" />
      </button>
      <nav aria-label={t("detail.breadcrumb")} className="min-w-0 flex-1">
        <ol className="flex items-center gap-1.5 text-xs text-muted-foreground min-w-0">
          {from ? (
            <>
              <li className="shrink-0">
                <Link
                  to={from.route}
                  className="hover:text-foreground hover:underline truncate"
                >
                  {from.pageLabel}
                </Link>
              </li>
              <li aria-hidden="true" className="text-muted-foreground shrink-0">
                ›
              </li>
              <li className="min-w-0 truncate text-foreground font-medium">
                {skillLabel}
              </li>
            </>
          ) : (
            <li className="min-w-0 truncate text-foreground font-medium">
              {skillLabel}
            </li>
          )}
        </ol>
      </nav>
    </div>
  );
}

/**
 * Route wrapper for `/skill/:skillId`.
 *
 * Reads the skill id from the URL and the optional originating page from
 * `location.state.from`, renders a `PageHeader` with back button + breadcrumb,
 * and delegates all content rendering / interactions to the shared
 * `SkillDetailView`.
 *
 * This component intentionally contains no AI explanation / install /
 * collection logic — those live in `SkillDetailView` so that the list-entry
 * drawer (`SkillDetailDrawer`, introduced in a sibling feature) can reuse
 * exactly the same behavior.
 */
export function SkillDetailPage() {
  const { skillId } = useParams<{ skillId: string }>();
  const navigate = useNavigate();
  const location = useLocation();

  // Pull the current skill's display name from the store so the breadcrumb
  // matches the rendered `<h1>` inside `SkillDetailView` once detail loads.
  const detailName = useSkillDetailStore((s) => s.detail?.name);

  const from = location.state?.from as SkillDetailPageFromState | undefined;
  const skillLabel = detailName ?? skillId ?? "";

  function handleGoBack() {
    // Preserve the pre-M4 return-scroll contract: if the caller packed a
    // scrollRestoration key into `location.state`, stash the offset before
    // popping back so the previous list can restore its scroll position.
    // The helpers remain in `src/lib/scrollRestoration.ts` (per the M4
    // design doc) so this path keeps working for any surface still
    // relying on it.
    const restorationState = location.state?.scrollRestoration;
    if (restorationState?.key) {
      saveScrollPosition(restorationState.key, restorationState.scrollTop ?? 0);
    }
    navigate(-1);
  }

  if (!skillId) {
    // Defensive: without a skill id in the URL we can't render anything
    // meaningful. Fall back to an empty shell — the router should normally
    // prevent this.
    return null;
  }

  return (
    <div className="flex flex-col h-full">
      <PageHeader from={from} skillLabel={skillLabel} onBack={handleGoBack} />
      <div className="flex-1 min-h-0">
        <SkillDetailView skillId={skillId} variant="page" leading={null} />
      </div>
    </div>
  );
}

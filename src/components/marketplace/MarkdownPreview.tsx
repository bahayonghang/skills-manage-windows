import { SkillMarkdownRenderer } from "@/components/skill/SkillMarkdownRenderer";
import { parseFrontmatter } from "@/lib/frontmatter";

interface MarkdownPreviewProps {
  content: string;
  className?: string;
}

export function MarkdownPreview({ content, className }: MarkdownPreviewProps) {
  const { body } = parseFrontmatter(content);
  return <SkillMarkdownRenderer content={body} variant="compact" className={className} />;
}

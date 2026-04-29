import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";

export interface SkillMarkdownRendererProps {
  content: string;
  variant?: "detail" | "compact";
  className?: string;
}

const markdownComponents: Components = {
  a({ href, className, children, ...props }) {
    const isExternal = typeof href === "string" && /^(https?:)?\/\//.test(href);

    return (
      <a
        {...props}
        href={href}
        className={cn(className)}
        target={isExternal ? "_blank" : props.target}
        rel={isExternal ? "noreferrer" : props.rel}
      >
        {children}
      </a>
    );
  },
  blockquote({ className, children, ...props }) {
    return (
      <blockquote
        {...props}
        className={cn(className)}
        data-skill-markdown-callout="true"
      >
        {children}
      </blockquote>
    );
  },
  pre({ className, children, ...props }) {
    return (
      <pre
        {...props}
        className={cn("scrollbar-subtle", className)}
        data-skill-markdown-pre="true"
      >
        {children}
      </pre>
    );
  },
  table({ className, children, ...props }) {
    return (
      <div
        className="skill-markdown-table scrollbar-subtle"
        data-skill-markdown-table="true"
      >
        <table {...props} className={cn(className)}>
          {children}
        </table>
      </div>
    );
  },
};

export function SkillMarkdownRenderer({
  content,
  variant = "detail",
  className,
}: SkillMarkdownRendererProps) {
  return (
    <article
      className={cn(
        "skill-markdown-panel",
        variant === "detail"
          ? "skill-markdown-panel--detail"
          : "skill-markdown-panel--compact",
        className
      )}
      data-skill-markdown-variant={variant}
    >
      {variant === "detail" ? (
        <div className="skill-markdown-panel__eyebrow">SKILL.md</div>
      ) : null}
      <div className="skill-markdown">
        <ReactMarkdown
          components={markdownComponents}
          remarkPlugins={[remarkGfm]}
        >
          {content}
        </ReactMarkdown>
      </div>
    </article>
  );
}

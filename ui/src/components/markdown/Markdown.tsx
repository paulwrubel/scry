import ReactMarkdown from 'react-markdown';
import type { Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';

const REMARK_PLUGINS = [remarkGfm];

const INLINE_ELEMENTS = ['a', 'br', 'code', 'del', 'em', 'strong'];

const COMPONENTS: Components = {
  a: ({ href, title, children }) => (
    <a href={href} title={title} target="_blank" rel="noreferrer">
      {children}
    </a>
  ),
  img: ({ src, alt, title }) => {
    if (typeof src !== 'string' || !/^https?:\/\//i.test(src)) {
      return alt === undefined || alt.length === 0 ? null : <span>{alt}</span>;
    }

    return (
      <img
        src={src}
        alt={alt ?? ''}
        title={title}
        loading="lazy"
        referrerPolicy="no-referrer"
        className="h-auto max-w-full rounded"
      />
    );
  },
  table: ({ children }) => (
    <div className="overflow-x-auto">
      <table>{children}</table>
    </div>
  ),
  // Descriptions live on pages that already own the h1, so markdown headings
  // start one level lower.
  h1: ({ children }) => <h2>{children}</h2>,
  h2: ({ children }) => <h3>{children}</h3>,
  h3: ({ children }) => <h4>{children}</h4>,
  h4: ({ children }) => <h5>{children}</h5>,
};

export type MarkdownProps = {
  children: string;
  variant?: 'block' | 'inline';
  className?: string;
};

export function Markdown(props: MarkdownProps) {
  const { children, variant = 'block', className } = props;

  const inline = variant === 'inline';
  const classes =
    `${inline ? '' : 'prose prose-sm prose-invert max-w-none'} ${className ?? ''}`.trim();

  return (
    <div className={classes}>
      <ReactMarkdown
        remarkPlugins={REMARK_PLUGINS}
        components={COMPONENTS}
        allowedElements={inline ? INLINE_ELEMENTS : undefined}
        unwrapDisallowed={inline}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}

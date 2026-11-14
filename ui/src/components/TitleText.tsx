import { Fragment } from 'react';
import { parseTitleSegments } from '@/utils/markdown/titleSegments';

export type TitleTextProps = {
  title: string;
};

export function TitleText(props: TitleTextProps) {
  const { title } = props;

  return (
    <>
      {parseTitleSegments(title).map((segment, index) =>
        segment.code ? (
          <code
            key={index}
            className="border-line-strong bg-panel rounded border px-1 py-0.5 font-mono text-[0.9em]"
          >
            {segment.text}
          </code>
        ) : (
          <Fragment key={index}>{segment.text}</Fragment>
        ),
      )}
    </>
  );
}

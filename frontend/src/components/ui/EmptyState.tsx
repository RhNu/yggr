interface Props {
  text: string;
}

export default function EmptyState({ text }: Props) {
  return <div className="py-8 text-center text-sm text-neutral-500">{text}</div>;
}

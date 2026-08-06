interface Props {
  text: string;
}

export default function EmptyState({ text }: Props) {
  return (
    <div className="text-center text-sm text-neutral-500 py-8">
      {text}
    </div>
  );
}

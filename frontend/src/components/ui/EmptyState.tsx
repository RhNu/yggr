interface Props {
  text: string;
}

export default function EmptyState({ text }: Props) {
  return (
    <div className="empty-state">
      <span>{text}</span>
    </div>
  );
}

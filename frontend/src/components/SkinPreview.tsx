import { useEffect, useRef } from "react";
import type { SkinViewer } from "skinview3d";

interface Props {
  skinUrl: string | null;
  capeUrl?: string | null;
  skinModel: "classic" | "slim";
}

export default function SkinPreview({ skinUrl, capeUrl, skinModel }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewerRef = useRef<SkinViewer | null>(null);

  useEffect(() => {
    if (!skinUrl || !containerRef.current) return;

    const url = skinUrl;
    const cape = capeUrl;
    let disposed = false;

    async function init() {
      if (!containerRef.current) return;
      const { SkinViewer } = await import("skinview3d");
      if (disposed || !containerRef.current) return;

      if (viewerRef.current) {
        viewerRef.current.dispose();
        viewerRef.current = null;
      }

      const viewer = new SkinViewer({
        width: containerRef.current.clientWidth,
        height: 300,
      });
      viewerRef.current = viewer;

      viewer.loadSkin(url, {
        model: skinModel === "slim" ? "slim" : "default",
      });

      if (cape) {
        viewer.loadCape(cape).catch(() => {});
      }

      containerRef.current.innerHTML = "";
      containerRef.current.appendChild(viewer.canvas);
    }

    init();

    return () => {
      disposed = true;
      if (viewerRef.current) {
        viewerRef.current.dispose();
        viewerRef.current = null;
      }
    };
  }, [skinUrl, capeUrl, skinModel]);

  if (!skinUrl) {
    return null;
  }

  return (
    <div
      ref={containerRef}
      className="skin-preview-container"
      style={{ minHeight: 300 }}
    />
  );
}

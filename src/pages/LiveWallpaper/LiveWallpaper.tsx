import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, fileSrc } from '../../api';

const IMAGE_EXTENSIONS = ['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'];

const fill: CSSProperties = {
  position: 'fixed',
  inset: 0,
  width: '100vw',
  height: '100vh',
  objectFit: 'cover',
  background: '#000',
  display: 'block',
};

/** Rendered inside the window that sits behind the desktop icons. */
export default function LiveWallpaper() {
  const [path, setPath] = useState<string | null>(null);
  const [paused, setPaused] = useState(false);
  const [volume, setVolume] = useState(0);
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    document.documentElement.style.background = '#000';
    document.body.style.background = '#000';
    api
      .liveStatus()
      .then((s) => {
        setPath(s.path);
        setPaused(s.paused);
        setVolume(s.volume);
      })
      .catch(console.error);

    const unlisteners = [
      listen<string>('live-src', (e) => setPath(e.payload)),
      listen<boolean>('live-pause', (e) => setPaused(e.payload)),
      listen<number>('live-volume', (e) => setVolume(e.payload)),
    ];
    return () => unlisteners.forEach((u) => u.then((f) => f()));
  }, []);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    video.volume = Math.min(1, volume / 100);
    video.muted = volume === 0;
    if (paused) {
      video.pause();
    } else {
      // Unmuted autoplay can be refused; fall back to muted playback.
      video.play().catch(() => {
        video.muted = true;
        video.play().catch(() => {});
      });
    }
  }, [paused, volume, path]);

  // Self-heal: Chromium can pause media while the desktop is covered or after sleep. Resume it,
  // so the wallpaper never just stops on its own.
  useEffect(() => {
    const kick = () => {
      const video = videoRef.current;
      if (video && !paused && video.paused) {
        video.play().catch(() => {
          video.muted = true;
          video.play().catch(() => {});
        });
      }
    };
    const timer = window.setInterval(kick, 4000);
    document.addEventListener('visibilitychange', kick);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener('visibilitychange', kick);
    };
  }, [paused]);

  if (!path) return <div style={fill} />;

  const src = fileSrc(path);
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  return IMAGE_EXTENSIONS.includes(ext) ? (
    <img key={src} src={src} style={fill} alt="" />
  ) : (
    <video ref={videoRef} key={src} src={src} style={fill} autoPlay loop muted playsInline disablePictureInPicture />
  );
}

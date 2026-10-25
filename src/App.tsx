import { useState, useEffect } from "react";
import {
  Monitor,
  AppWindow,
  Crop,
  Mic,
  Volume2,
  Camera,
  Settings,
  Square,
  Play,
  Pause,
  FolderOpen,
  CheckCircle,
  Video,
  MousePointer,
  Zap,
} from "lucide-react";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  if (cmd === "get_monitors") {
    return [
      { id: 0, name: "Main Display (1920x1080)", width: 1920, height: 1080, isPrimary: true, scaleFactor: 1.0 },
    ] as unknown as T;
  }
  if (cmd === "get_windows") {
    return [
      { id: "1", title: "Visual Studio Code", appName: "Code", x: 0, y: 0, width: 1920, height: 1080, isMinimized: false },
      { id: "2", title: "Google Chrome", appName: "Chrome", x: 0, y: 0, width: 1280, height: 720, isMinimized: false },
    ] as unknown as T;
  }
  if (cmd === "get_hardware_capabilities") {
    return {
      hasNvenc: true,
      hasQuicksync: false,
      hasAmf: false,
      hasVaapi: false,
      ffmpegInstalled: true,
      ffmpegPath: "ffmpeg",
    } as unknown as T;
  }
  return {} as T;
}

interface MonitorInfo {
  id: number;
  name: string;
  width: number;
  height: number;
  isPrimary: boolean;
  scaleFactor: number;
}

interface WindowInfo {
  id: string;
  title: string;
  appName: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

interface CompletedInfo {
  filePath: string;
  durationSeconds: number;
  fileSizeBytes: number;
}

export default function App() {
  const [captureMode, setCaptureMode] = useState<"full" | "window" | "area">("full");
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [selectedMonitor, setSelectedMonitor] = useState<number>(0);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [selectedWindow, setSelectedWindow] = useState<string>("");

  const [areaRegion, setAreaRegion] = useState({ x: 100, y: 100, width: 1280, height: 720 });

  const [micEnabled, setMicEnabled] = useState(true);
  const [systemAudioEnabled, setSystemAudioEnabled] = useState(true);
  const [webcamEnabled, setWebcamEnabled] = useState(false);
  const [showMouseClicks, setShowMouseClicks] = useState(true);

  const [fps, setFps] = useState(60);
  const [format, setFormat] = useState("mp4");
  const [quality, setQuality] = useState("high");
  const [countdownSecs, setCountdownSecs] = useState(3);
  const [showSettings, setShowSettings] = useState(false);

  const [isRecording, setIsRecording] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [countdownRemaining, setCountdownRemaining] = useState<number | null>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [micLevelDb, setMicLevelDb] = useState(-60);
  const [completedResult, setCompletedResult] = useState<CompletedInfo | null>(null);

  useEffect(() => {
    tauriInvoke<MonitorInfo[]>("get_monitors").then((res) => {
      if (res && res.length > 0) setMonitors(res);
    });
    tauriInvoke<WindowInfo[]>("get_windows").then((res) => {
      if (res && res.length > 0) {
        setWindows(res);
        setSelectedWindow(res[0].id);
      }
    });

    if (isTauri) {
      import("@tauri-apps/api/event").then(({ listen }) => {
        listen<{ elapsedSeconds: number; micDb: number }>("telemetry-tick", (event) => {
          setElapsedSeconds(event.payload.elapsedSeconds);
          setMicLevelDb(event.payload.micDb);
        });
        listen<{ status: string; remainingSeconds?: number }>("recording-state-changed", (event) => {
          if (event.payload.status === "countdown") {
            setCountdownRemaining(event.payload.remainingSeconds ?? 3);
          } else if (event.payload.status === "recording") {
            setCountdownRemaining(null);
            setIsRecording(true);
            setIsPaused(false);
          }
        });
      });
    }
  }, []);

  const formatTime = (secs: number) => {
    const total = Math.floor(secs);
    const m = Math.floor(total / 60);
    const s = total % 60;
    const pad = (n: number) => n.toString().padStart(2, "0");
    return `${pad(m)}:${pad(s)}`;
  };

  const dbToPercent = (db: number) => {
    if (db <= -60) return 0;
    return Math.min(100, Math.max(0, ((db + 60) / 60) * 100));
  };

  const handleStartRecording = async () => {
    setCompletedResult(null);

    const currentConfig = {
      target: {
        mode: captureMode === "full" ? "fullScreen" : captureMode === "window" ? "window" : "area",
        monitorIndex: selectedMonitor,
        windowId: selectedWindow || null,
        region: captureMode === "area" ? areaRegion : null,
      },
      video: {
        fps,
        showCursor: true,
        quality,
        encoder: "auto",
        scaleFactor: 1.0,
      },
      audio: {
        micEnabled,
        micVolume: 1.0,
        systemAudioEnabled,
        systemVolume: 1.0,
      },
      webcam: {
        enabled: webcamEnabled,
        shape: "circle",
        mirror: true,
      },
      effects: {
        showMouseClicks,
        magnifierEnabled: false,
      },
      output: {
        format,
        countdownSeconds: countdownSecs,
      },
    };

    try {
      await tauriInvoke("save_config", { config: currentConfig });

      if (countdownSecs > 0) {
        setCountdownRemaining(countdownSecs);
        const timer = setInterval(() => {
          setCountdownRemaining((prev) => {
            if (prev === null || prev <= 1) {
              clearInterval(timer);
              setCountdownRemaining(null);
              setIsRecording(true);
              return null;
            }
            return prev - 1;
          });
        }, 1000);
      } else {
        setIsRecording(true);
      }

      await tauriInvoke("start_recording");
    } catch (err) {
      console.error("Start recording failed:", err);
    }
  };

  const handleTogglePause = async () => {
    try {
      if (isPaused) {
        await tauriInvoke("resume_recording");
        setIsPaused(false);
      } else {
        await tauriInvoke("pause_recording");
        setIsPaused(true);
      }
    } catch (err) {
      console.error("Toggle pause failed:", err);
    }
  };

  const handleStopRecording = async () => {
    try {
      const res = await tauriInvoke<CompletedInfo>("stop_recording");
      setIsRecording(false);
      setIsPaused(false);
      setElapsedSeconds(0);
      if (res && res.filePath) {
        setCompletedResult(res);
      }
    } catch (err) {
      console.error("Stop recording failed:", err);
      setIsRecording(false);
    }
  };

  const handleOpenFolder = (path: string) => {
    tauriInvoke("open_recording_folder", { path });
  };

  const handleOpenFile = (path: string) => {
    tauriInvoke("open_recording_file", { path });
  };

  return (
    <div className="app-container">
      
      {countdownRemaining !== null && (
        <div className="countdown-overlay">
          <div className="countdown-number">{countdownRemaining}</div>
          <div className="countdown-hint">Get ready to record...</div>
        </div>
      )}

      
      <header className="app-header">
        <div className="brand">
          <div className="brand-icon">
            <Video size={22} />
          </div>
          <div className="brand-title">Lumaq</div>
          <div className="brand-badge">
            <Zap size={12} style={{ display: "inline", marginRight: 4 }} />
            NVENC Ready
          </div>
        </div>

        <div className="header-actions">
          <button
            className={`icon-btn ${showSettings ? "active" : ""}`}
            onClick={() => setShowSettings(!showSettings)}
            title="Recording Settings"
          >
            <Settings size={18} />
          </button>
        </div>
      </header>

      
      {completedResult && (
        <div className="completed-banner">
          <div className="completed-info">
            <div className="completed-title">
              <CheckCircle size={16} style={{ display: "inline", marginRight: 6 }} />
              Recording Saved Successfully!
            </div>
            <div className="completed-path">{completedResult.filePath}</div>
          </div>
          <div style={{ display: "flex", gap: 8 }}>
            <button
              className="btn-secondary"
              onClick={() => handleOpenFile(completedResult.filePath)}
            >
              <Play size={14} /> Play
            </button>
            <button
              className="btn-secondary"
              onClick={() => handleOpenFolder(completedResult.filePath)}
            >
              <FolderOpen size={14} /> Folder
            </button>
          </div>
        </div>
      )}

      
      {showSettings && (
        <div className="settings-drawer">
          <div style={{ fontWeight: 600, fontSize: "0.95rem" }}>Recording Configuration</div>
          <div className="settings-grid">
            <div className="setting-item">
              <label className="setting-label">Frame Rate (FPS)</label>
              <select
                className="setting-select"
                value={fps}
                onChange={(e) => setFps(Number(e.target.value))}
              >
                <option value={30}>30 FPS (Standard)</option>
                <option value={60}>60 FPS (Silky Smooth)</option>
                <option value={120}>120 FPS (High Refresh Gaming)</option>
              </select>
            </div>

            <div className="setting-item">
              <label className="setting-label">Video Format</label>
              <select
                className="setting-select"
                value={format}
                onChange={(e) => setFormat(e.target.value)}
              >
                <option value="mp4">MP4 (FastStart / Universal)</option>
                <option value="mkv">MKV (Crash Resilient)</option>
                <option value="webm">WebM (VP9 Web Format)</option>
                <option value="gif">Animated GIF (Pure Rust)</option>
              </select>
            </div>

            <div className="setting-item">
              <label className="setting-label">Quality Preset</label>
              <select
                className="setting-select"
                value={quality}
                onChange={(e) => setQuality(e.target.value)}
              >
                <option value="ultra">Ultra (Visually Lossless)</option>
                <option value="high">High (Recommended)</option>
                <option value="medium">Medium (Compact File Size)</option>
              </select>
            </div>

            <div className="setting-item">
              <label className="setting-label">Pre-roll Countdown</label>
              <select
                className="setting-select"
                value={countdownSecs}
                onChange={(e) => setCountdownSecs(Number(e.target.value))}
              >
                <option value={0}>Instant (No Countdown)</option>
                <option value={3}>3 Seconds</option>
                <option value={5}>5 Seconds</option>
              </select>
            </div>
          </div>
        </div>
      )}

      
      <section>
        <div className="section-label">1. Choose Capture Target</div>
        <div className="target-modes">
          <div
            className={`target-card ${captureMode === "full" ? "active" : ""}`}
            onClick={() => setCaptureMode("full")}
          >
            <Monitor size={28} className="target-card-icon" />
            <div className="target-card-title">Full Screen</div>
            <div className="target-card-desc">Capture entire display</div>
          </div>

          <div
            className={`target-card ${captureMode === "window" ? "active" : ""}`}
            onClick={() => setCaptureMode("window")}
          >
            <AppWindow size={28} className="target-card-icon" />
            <div className="target-card-title">Window</div>
            <div className="target-card-desc">Single app or browser</div>
          </div>

          <div
            className={`target-card ${captureMode === "area" ? "active" : ""}`}
            onClick={() => setCaptureMode("area")}
          >
            <Crop size={28} className="target-card-icon" />
            <div className="target-card-title">Select Area</div>
            <div className="target-card-desc">Custom rectangle snip</div>
          </div>
        </div>

        
        <div style={{ marginTop: 12 }}>
          {captureMode === "full" && monitors.length > 1 && (
            <div className="sub-picker">
              <Monitor size={16} color="#818cf8" />
              <select
                value={selectedMonitor}
                onChange={(e) => setSelectedMonitor(Number(e.target.value))}
              >
                {monitors.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name} ({m.width}x{m.height}) {m.isPrimary ? "[Primary]" : ""}
                  </option>
                ))}
              </select>
            </div>
          )}

          {captureMode === "window" && (
            <div className="sub-picker">
              <AppWindow size={16} color="#818cf8" />
              <select
                value={selectedWindow}
                onChange={(e) => setSelectedWindow(e.target.value)}
              >
                {windows.length > 0 ? (
                  windows.map((w) => (
                    <option key={w.id} value={w.id}>
                      {w.title} ({w.appName})
                    </option>
                  ))
                ) : (
                  <option value="">No open application windows detected</option>
                )}
              </select>
            </div>
          )}

          {captureMode === "area" && (
            <div className="sub-picker" style={{ justifyContent: "space-between" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 12, fontSize: "0.85rem" }}>
                <Crop size={16} color="#818cf8" />
                <span>Region: <b>{areaRegion.width} × {areaRegion.height} px</b></span>
                <span style={{ color: "var(--text-muted)" }}>(Offset: {areaRegion.x}, {areaRegion.y})</span>
              </div>
              <button
                className="btn-secondary"
                style={{ padding: "6px 14px", fontSize: "0.8rem" }}
                onClick={() => {
                  setAreaRegion({ x: 150, y: 150, width: 1280, height: 720 });
                }}
              >
                Reset 720p
              </button>
            </div>
          )}
        </div>
      </section>

      
      <section>
        <div className="section-label">2. Audio & Overlay Options</div>
        <div className="toggles-grid">
          
          <div className="toggle-card">
            <div className="toggle-header">
              <div className="toggle-label">
                <Mic size={18} color={micEnabled ? "#10b981" : "var(--text-muted)"} />
                Microphone
              </div>
              <label className="switch">
                <input
                  type="checkbox"
                  checked={micEnabled}
                  onChange={(e) => setMicEnabled(e.target.checked)}
                />
                <span className="slider"></span>
              </label>
            </div>
            <div className="vu-meter">
              <div
                className="vu-bar"
                style={{
                  width: micEnabled ? `${dbToPercent(micLevelDb)}%` : "0%",
                }}
              ></div>
            </div>
          </div>

          
          <div className="toggle-card">
            <div className="toggle-header">
              <div className="toggle-label">
                <Volume2 size={18} color={systemAudioEnabled ? "#38bdf8" : "var(--text-muted)"} />
                Desktop Sound
              </div>
              <label className="switch">
                <input
                  type="checkbox"
                  checked={systemAudioEnabled}
                  onChange={(e) => setSystemAudioEnabled(e.target.checked)}
                />
                <span className="slider"></span>
              </label>
            </div>
            <div className="vu-meter">
              <div
                className="vu-bar"
                style={{ width: systemAudioEnabled ? "15%" : "0%" }}
              ></div>
            </div>
          </div>

          
          <div className="toggle-card">
            <div className="toggle-header">
              <div className="toggle-label">
                <Camera size={18} color={webcamEnabled ? "#a855f7" : "var(--text-muted)"} />
                Webcam PiP
              </div>
              <label className="switch">
                <input
                  type="checkbox"
                  checked={webcamEnabled}
                  onChange={(e) => setWebcamEnabled(e.target.checked)}
                />
                <span className="slider"></span>
              </label>
            </div>
            <div style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
              {webcamEnabled ? "Circular Floating Avatar active" : "Camera overlay disabled"}
            </div>
          </div>

          
          <div className="toggle-card">
            <div className="toggle-header">
              <div className="toggle-label">
                <MousePointer size={18} color={showMouseClicks ? "#38bdf8" : "var(--text-muted)"} />
                Click Ripple
              </div>
              <label className="switch">
                <input
                  type="checkbox"
                  checked={showMouseClicks}
                  onChange={(e) => setShowMouseClicks(e.target.checked)}
                />
                <span className="slider"></span>
              </label>
            </div>
            <div style={{ fontSize: "0.75rem", color: "var(--text-muted)" }}>
              {showMouseClicks ? "Animated ripple on click" : "Clicks invisible"}
            </div>
          </div>
        </div>
      </section>

      
      <section className="hero-dock">
        <div className="dock-left">
          <div className="timer-display">
            {isRecording && <div className="rec-pulse-dot" />}
            {isRecording ? formatTime(elapsedSeconds) : "00:00"}
          </div>
          <div className="dock-meta">
            <span>Target: <b>{fps} FPS</b></span>
            <span>Format: <b>{format.toUpperCase()}</b></span>
            <span>Mode: <b>{quality.toUpperCase()}</b></span>
          </div>
        </div>

        <div className="dock-actions">
          {!isRecording ? (
            <button className="btn-record" onClick={handleStartRecording}>
              <div
                style={{
                  width: 12,
                  height: 12,
                  borderRadius: "50%",
                  background: "white",
                }}
              />
              Start Recording
            </button>
          ) : (
            <>
              <button className="btn-secondary" onClick={handleTogglePause}>
                {isPaused ? <Play size={16} /> : <Pause size={16} />}
                {isPaused ? "Resume" : "Pause"}
              </button>

              <button
                className="btn-record"
                style={{
                  background: "linear-gradient(135deg, #1e293b 0%, #0f172a 100%)",
                  boxShadow: "0 0 20px rgba(0,0,0,0.4)",
                  border: "1px solid rgba(255,255,255,0.15)",
                }}
                onClick={handleStopRecording}
              >
                <Square size={16} fill="white" />
                Stop & Save
              </button>
            </>
          )}
        </div>
      </section>
    </div>
  );
}

import os
import time
import subprocess
import shutil
import socket
from pathlib import Path


def get_user_cache_dir() -> Path:
    """Get platform-appropriate cache directory"""
    if os.name == "nt":  # Windows
        return Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local"))
    elif "XDG_CACHE_HOME" in os.environ:  # Linux with XDG
        return Path(os.environ["XDG_CACHE_HOME"])
    else:  # macOS and fallback
        return Path.home() / ".cache"


def _can_use_pgrep() -> bool:
    return os.name != "nt" and shutil.which("pgrep") is not None


def _wait_for_socket(sock_path: Path, timeout_s: float = 2.5) -> None:
    """
    Wait until the mpv IPC socket exists and is connectable.
    Prevents a race where mpv is launched but not ready yet.
    """
    deadline = time.time() + timeout_s
    last_err = None
    while time.time() < deadline:
        if sock_path.exists():
            try:
                s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                s.settimeout(0.25)
                s.connect(str(sock_path))
                s.close()
                return
            except Exception as e:
                last_err = e
        time.sleep(0.05)
    if last_err:
        raise RuntimeError(f"mpv socket not ready: {last_err}")
    raise RuntimeError("mpv socket not ready (timeout)")


def ensure_mpv_running() -> Path:
    cache_dir = get_user_cache_dir()
    sock = cache_dir / "bangtunes" / "mpv.sock"
    sock.parent.mkdir(parents=True, exist_ok=True)

    # If mpv already running with our socket, we're done
    try:
        if _can_use_pgrep():
            out = subprocess.run(
                ["pgrep", "-af", "mpv"], capture_output=True, text=True, check=False
            )
            if str(sock) in out.stdout:
                return sock
    except Exception:
        # Non-fatal: just fall through and start mpv
        pass

    # Remove stale socket
    try:
        sock.unlink()
    except FileNotFoundError:
        pass

    # Start mpv detached
    subprocess.Popen(
        [
            "mpv",
            "--idle=yes",
            "--no-video",
            "--no-terminal",
            f"--input-ipc-server={sock}",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    # Wait up to ~5s for socket (slow first-start happens)
    timeout_s = float(os.environ.get("BANGTUNES_MPV_START_TIMEOUT", "5.0"))
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if sock.exists():
            # Socket exists, now wait for it to be connectable
            try:
                _wait_for_socket(sock, timeout_s=min(2.5, deadline - time.time()))
                return sock
            except RuntimeError:
                # Continue waiting if socket isn't ready yet
                pass
        time.sleep(0.05)

    raise RuntimeError(f"mpv IPC socket did not appear at {sock}")

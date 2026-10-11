"""Measure the real downloader against an isolated local media fixture."""
import concurrent.futures
import copy
import http.server
import json
import os
import pathlib
import shutil
import statistics
import subprocess
import tempfile
import threading
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
YTDLP = ROOT / 'src-tauri/target/release/tools/yt-dlp.exe'
FFMPEG = pathlib.Path(os.environ.get('YDLITE_FFMPEG') or shutil.which('ffmpeg') or
                      ROOT / 'src-tauri/target/release/tools/ffmpeg/ffmpeg.exe')
HIDDEN = subprocess.CREATE_NO_WINDOW if hasattr(subprocess, 'CREATE_NO_WINDOW') else 0


def run(args, **kwargs):
    start = time.perf_counter()
    result = subprocess.run(args, capture_output=True, text=True, encoding='utf-8',
                            timeout=30, creationflags=HIDDEN, **kwargs)
    return result, time.perf_counter() - start


probes = [(str(YTDLP), '--version'), (str(FFMPEG), '-version'),
          (str(FFMPEG.with_name('ffprobe.exe')), '-version')]
serial, parallel = [], []
for _ in range(3):
    start = time.perf_counter()
    assert all(run(command)[0].returncode == 0 for command in probes)
    serial.append(time.perf_counter() - start)
    start = time.perf_counter()
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        assert all(result[0].returncode == 0 for result in pool.map(run, probes))
    parallel.append(time.perf_counter() - start)
print(f'Tool probes: serial {statistics.median(serial):.3f}s, '
      f'parallel {statistics.median(parallel):.3f}s', flush=True)

assets = {}
hits = {'page': 0, 'media': 0}


class Fixture(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        if self.path == '/page':
            hits['page'] += 1
            time.sleep(0.6)  # A deterministic metadata round trip.
            body = b'<html><title>Performance fixture</title><video src="/media.mp4"></video></html>'
            content_type = 'text/html'
        elif self.path == '/media.mp4':
            hits['media'] += 1
            body, content_type = assets['media'], 'video/mp4'
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header('Content-Type', content_type)
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        try:
            self.wfile.write(body)
        except (BrokenPipeError, ConnectionError, OSError):
            pass


with tempfile.TemporaryDirectory(prefix='ydlite-performance-') as directory:
    directory = pathlib.Path(directory)
    media = directory / 'fixture.mp4'
    result, _ = run([str(FFMPEG), '-loglevel', 'error', '-f', 'lavfi', '-i',
                     'testsrc=size=64x64:rate=5', '-t', '1', '-c:v', 'mpeg4', str(media)])
    assert result.returncode == 0, 'fixture generation failed'
    assets['media'] = media.read_bytes()
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        url = f'http://127.0.0.1:{server.server_port}/page'
        first, shallow_time = run([str(YTDLP), '--ignore-config', '-J', '--flat-playlist', url])
        assert first.returncode == 0, 'shallow parsing failed'
        info = json.loads(first.stdout)
        assert info.get('formats') and all(fmt.get('url') for fmt in info['formats'])
        assert info.get('_type') in (None, 'video') and info.get('extractor') in ('generic', 'html5')
        assert info.get('id') and info.get('title')
        assert all(f.get('protocol') in ('http', 'https') and f.get('ext') == 'mp4' for f in info['formats'])
        deep, deep_time = run([str(YTDLP), '--ignore-config', '-J', '--no-playlist', url])
        assert deep.returncode == 0
        deep_info = json.loads(deep.stdout)
        assert [(f['format_id'], f['url']) for f in info['formats']] == [
            (f['format_id'], f['url']) for f in deep_info['formats']]
        print(f'Analyze fixture: duplicate {shallow_time + deep_time:.3f}s, '
              f'single {shallow_time:.3f}s; same downloadable formats', flush=True)
        download_args = [str(YTDLP), '--ignore-config', '--no-playlist', '--no-simulate']
        page_before = hits['page']
        direct, direct_time = run([*download_args, '-o', str(directory/'direct.mp4'), url])
        assert direct.returncode == 0
        assert hits['page'] > page_before
        page_before = hits['page']
        cached, cached_time = run([*download_args, '--load-info-json', '-', '-o',
                                   str(directory/'cached.mp4')], input=json.dumps(info))
        assert cached.returncode == 0, 'metadata stdin download failed'
        assert hits['page'] == page_before, 'cached download unexpectedly extracted metadata'
        assert (directory/'cached.mp4').read_bytes() == media.read_bytes()
        print(f'Download fixture: URL {direct_time:.3f}s, metadata stdin {cached_time:.3f}s; '
              'same output, no second metadata request', flush=True)
        stale = copy.deepcopy(info)
        for fmt in stale['formats']:
            fmt['url'] = f'http://127.0.0.1:{server.server_port}/expired.mp4'
        stale['url'] = f'http://127.0.0.1:{server.server_port}/expired.mp4'
        stale_result, _ = run([*download_args, '--load-info-json', '-', '-o',
                               str(directory/'fallback.mp4')], input=json.dumps(stale))
        print(f'Expired metadata: exit {stale_result.returncode}, '
              f'fallback output {(directory/"fallback.mp4").is_file()}', flush=True)
    finally:
        server.shutdown()
        server.server_close()

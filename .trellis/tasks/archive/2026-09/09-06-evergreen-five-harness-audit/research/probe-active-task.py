"""Read-only repository audit: reproduce session routing using owned temp fixtures."""
import json
import sys
import tempfile
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / '.trellis' / 'scripts'))
from common.active_task import clear_active_task, resolve_active_task

with tempfile.TemporaryDirectory(prefix='skillport-harness-audit-') as temp:
    repo = Path(temp)
    sessions = repo / '.trellis' / '.runtime' / 'sessions'
    sessions.mkdir(parents=True)
    old_session = sessions / 'codex_old.json'
    task_ref = '.trellis/tasks/01-01-old'
    old_session.write_text(json.dumps({'current_task': task_ref}), encoding='utf-8')
    with patch('common.active_task.resolve_context_key', return_value='codex_new'):
        stale = resolve_active_task(repo)
        print(json.dumps({'case': 'new-session-missing-task', **stale.__dict__}))
        (repo / task_ref).mkdir(parents=True)
        (repo / task_ref / 'task.json').write_text('{"status":"planning"}', encoding='utf-8')
        active = resolve_active_task(repo)
        print(json.dumps({'case': 'new-session-borrows-live-task', **active.__dict__}))
        cleared = clear_active_task(repo)
        print(json.dumps({'case': 'clear-new-session', 'cleared_source': cleared.source,
                          'other_session_survives': old_session.exists()}))

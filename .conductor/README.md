# Conductor Setup for connections+ Worktrees

This conductor configuration enables testing the connections+ UI across multiple worktrees on different ports while sharing a single database.

## Configuration

### Scripts

- **setup.sh** — Verifies Rust toolchain and sqlx-cli, runs once per worktree
- **archive.sh** — Validates archive.json availability
- **run.sh** — Starts dev server on auto-detected available port, sharing DATABASE_URL

### Environment

Database and archive paths are shared across all worktrees via `settings.local.toml`:

```toml
DATABASE_URL = "sqlite:///Users/jakehash/Development/articles/connections+/games.db?mode=rwc"
ARCHIVE_PATH = "/Users/jakehash/Development/articles/connections+/archive.json"
```

## Usage

### First-time setup
```bash
conductor setup
```

### Run a worktree
```bash
conductor run
```

The dev server will:
1. Auto-find first available port starting from 3062
2. Print the port to stdout (e.g., "Starting dev server on port 3063")
3. Use shared games.db and archive.json
4. Hot-reload on code changes via cargo watch

### Multiple worktrees

Each worktree in `/Users/jakehash/conductor/workspaces/connections+/` gets:
- Independent source tree (git branch/state)
- Same database connection (all instances access same data)
- Different port (auto-assigned to avoid conflicts)

### Troubleshooting

**Port Detection**
- Uses `nc -z localhost $PORT` to check availability
- Increments from 3062 until it finds a free port
- Max 100 attempts before failing

**Database Not Loading Latest Data**
- Verify `DATABASE_URL` is correctly set and accessible
- Check file permissions: `ls -la /Users/jakehash/Development/articles/connections+/games.db`
- Ensure migrations run: look for migration output at startup
- Test connection: `sqlite3 sqlite:///path/to/games.db "SELECT COUNT(*) FROM puzzles;"`

**Worktree Isolation**
- Source code is isolated per worktree (different branches/commits)
- Database state is shared across all worktrees
- To test with isolated DB: set custom `DATABASE_URL` per worktree (override in `.conductor/settings.local.toml`)

## Architecture

```
.conductor/
├── settings.local.toml   # Shared config & env vars
├── scripts/
│   ├── setup.sh         # Initialize worktree
│   ├── run.sh           # Start dev server (auto-port)
│   └── archive.sh       # Validate archive
└── README.md

/Users/jakehash/conductor/workspaces/connections+/
├── branch-1/            # Worktree 1 (git branch)
│   └── (isolated source)
├── branch-2/            # Worktree 2 (git branch)
│   └── (isolated source)
└── ...

/Users/jakehash/Development/articles/connections+/
├── games.db             # Shared across all worktrees
└── archive.json         # Shared across all worktrees
```

## Notes

- Migrations run automatically on server startup (via sqlx::migrate! in main.rs)
- systemfd persists socket across recompiles (no connection reset)
- cargo watch watches file system, triggers rebuild on changes
- Environment variables in `settings.local.toml [env]` section are loaded by conductor

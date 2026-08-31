import { DatabaseSync } from 'node:sqlite';
import { mkdirSync, readdirSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { QA_SCHEMA_VERSION } from './constants.mjs';

const MIGRATIONS_DIR = fileURLToPath(new URL('./migrations/', import.meta.url));

export function defaultDatabasePath(env = process.env, platform = process.platform) {
  if (env.AUTODRILL_QA_DB_PATH) return resolve(env.AUTODRILL_QA_DB_PATH);
  if (platform === 'darwin') return join(homedir(), 'Library', 'Application Support', 'AutoDrill', 'qa.sqlite3');
  if (platform === 'win32') return join(env.LOCALAPPDATA ?? join(homedir(), 'AppData', 'Local'), 'AutoDrill', 'qa.sqlite3');
  return join(env.XDG_DATA_HOME ?? join(homedir(), '.local', 'share'), 'autodrill', 'qa.sqlite3');
}

export function migrationFiles() {
  return readdirSync(MIGRATIONS_DIR)
    .filter((name) => /^\d{3}_.+[.]sql$/.test(name))
    .sort()
    .map((name) => ({
      name,
      version: Number(name.slice(0, 3)),
      sql: readFileSync(join(MIGRATIONS_DIR, name), 'utf8'),
    }));
}

export function openDatabase({ path = defaultDatabasePath(), maxMigration = QA_SCHEMA_VERSION } = {}) {
  mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
  const database = new DatabaseSync(path);
  database.exec('PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA busy_timeout = 5000;');
  database.exec(`
    CREATE TABLE IF NOT EXISTS schema_migrations (
      version INTEGER PRIMARY KEY,
      name TEXT NOT NULL UNIQUE,
      applied_at TEXT NOT NULL
    ) STRICT;
  `);
  const applied = new Set(database.prepare('SELECT version FROM schema_migrations').all().map((row) => row.version));
  const migrations = migrationFiles().filter((migration) => migration.version <= maxMigration);
  for (const migration of migrations) {
    if (applied.has(migration.version)) continue;
    database.exec('BEGIN IMMEDIATE');
    try {
      database.exec(migration.sql);
      database.prepare('INSERT INTO schema_migrations(version, name, applied_at) VALUES (?, ?, ?)')
        .run(migration.version, migration.name, new Date().toISOString());
      database.exec(`PRAGMA user_version = ${migration.version}`);
      database.exec('COMMIT');
    } catch (error) {
      database.exec('ROLLBACK');
      database.close();
      throw new Error(`Migration ${migration.name} failed: ${error.message}`, { cause: error });
    }
  }
  const current = database.prepare('SELECT COALESCE(MAX(version), 0) AS version FROM schema_migrations').get().version;
  if (current > QA_SCHEMA_VERSION) {
    database.close();
    throw new Error(`Database schema ${current} is newer than this QA application (${QA_SCHEMA_VERSION}).`);
  }
  return { database, path, schemaVersion: current };
}

export function transaction(database, operation) {
  database.exec('BEGIN IMMEDIATE');
  try {
    const result = operation();
    database.exec('COMMIT');
    return result;
  } catch (error) {
    database.exec('ROLLBACK');
    throw error;
  }
}

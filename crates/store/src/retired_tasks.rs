//! Shipped migrations for the retired Tasks feature (ADR 0031). SQL is unchanged.

/// Migration 14. Shipped: never edit it.
pub(crate) const TABLE: &str = "
    CREATE TABLE tasks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        created_ms INTEGER NOT NULL,
        updated_ms INTEGER NOT NULL,
        goal TEXT NOT NULL,
        roost TEXT NOT NULL DEFAULT '',
        roost_name TEXT NOT NULL DEFAULT '',
        folder TEXT NOT NULL DEFAULT '',
        owner TEXT NOT NULL DEFAULT 'you',
        status TEXT NOT NULL DEFAULT 'open',
        cost_usd REAL,
        issue TEXT
    );
    CREATE INDEX tasks_status ON tasks (status, updated_ms);
    CREATE TABLE task_criteria (
        task INTEGER NOT NULL,
        idx INTEGER NOT NULL,
        text TEXT NOT NULL,
        done INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (task, idx)
    );";

/// Migration 15: what the last session Vults started for it said, and its session id, so the next
/// **Start** goes on with it (W4). Shipped: never edit it.
pub(crate) const RUNS: &str = "
    ALTER TABLE tasks ADD COLUMN reply TEXT;
    ALTER TABLE tasks ADD COLUMN session TEXT;";

/// Migration 16: the context pack its last handoff left (W5). Shipped: never edit it.
pub(crate) const PACK: &str = "ALTER TABLE tasks ADD COLUMN pack TEXT;";

/// Migration 17: the last review of its work (W6): what the reviewer said, its verdict (`pass`,
/// `changes`, or `none` when it gave none) and the round. Shipped: never edit it.
pub(crate) const REVIEW: &str = "
    ALTER TABLE tasks ADD COLUMN review TEXT;
    ALTER TABLE tasks ADD COLUMN review_verdict TEXT;
    ALTER TABLE tasks ADD COLUMN review_round INTEGER;";

/// Migration 18: why the router sent it where it went (W8): `rule:<agent>`, `default:<agent>`,
/// `quota:<agent>:<busy>:<used>` or `score:<agent>:<done>:<of>`. Shipped: never edit it.
pub(crate) const ROUTE: &str = "ALTER TABLE tasks ADD COLUMN route TEXT;";

/// Migration 20: the tokens its runs used, for an agent that reports no dollars (Codex; W9).
/// Shipped: never edit it.
pub(crate) const TOKENS: &str = "ALTER TABLE tasks ADD COLUMN tokens INTEGER;";

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn shipped_sql_is_unchanged() {
        for (sql, expected) in [
            (
                crate::spend::TABLE,
                "88d6c19fd15edce9337e072a1845a79fb7032e02920a04ff9f98365451032d3c",
            ),
            (
                TABLE,
                "ab89883df72d89a2b1352a0b1e017f08411dbadae653d20928d06de9c4139d24",
            ),
            (
                RUNS,
                "73dfc913ed08706ba7d295e011cd5ad88ba86ef792cad1cc05e39d5ee3c02366",
            ),
            (
                PACK,
                "0e8e174606ff99ce3eb561a2618b80318c5846d7d6671cae628493673c278108",
            ),
            (
                REVIEW,
                "9c1e56ddd2cf3fae10b1e7dc67a7b9a1a37d9d2b4b589996fef0f8dd6187c850",
            ),
            (
                ROUTE,
                "e57985beb82f107532cf8e25cb7e7608a8bf4d6bbbc22d9c5e06a9aaeced528e",
            ),
            (
                TOKENS,
                "0446a286127490d028d99c56c53025411fe4146f24089179325329c11fc4f08b",
            ),
        ] {
            assert_eq!(format!("{:x}", Sha256::digest(sql.as_bytes())), expected);
        }
    }
}

-- 001 — One consumption row per (student, product, day).
--
-- Legacy data has duplicate rows for the same cell because the Go app's
-- upsert was not transactional. This script merges each group into its
-- oldest row (summing quantities), adds a UNIQUE index so duplicates can
-- never appear again, and rebuilds the payments index found corrupt in
-- production (`PRAGMA integrity_check`: "row 226 missing from index").
--
-- Precondition: rows of a duplicate group share the same sale price, so
-- merging does not change any total. Verify it first:
--   SELECT COUNT(*) FROM (
--     SELECT 1 FROM consumos GROUP BY id_estudiante, id_producto, fecha_consumo
--     HAVING COUNT(*) > 1 AND COUNT(DISTINCT precio_unitario_venta) > 1);
--   -- must return 0
--
-- Idempotent: running it again changes nothing.
-- Usage: sqlite3 database/database.db < migrations/001_consumos_unique.sql
-- (stop the app first and keep a backup).

BEGIN IMMEDIATE;

-- 1. The oldest row of each duplicate group keeps the summed quantity.
UPDATE consumos
SET cantidad = (
    SELECT SUM(d.cantidad)
    FROM consumos d
    WHERE d.id_estudiante = consumos.id_estudiante
      AND d.id_producto = consumos.id_producto
      AND d.fecha_consumo = consumos.fecha_consumo
)
WHERE id_consumo IN (
    SELECT MIN(id_consumo)
    FROM consumos
    GROUP BY id_estudiante, id_producto, fecha_consumo
    HAVING COUNT(*) > 1
);

-- 2. Remove the other rows of those groups.
DELETE FROM consumos
WHERE id_consumo NOT IN (
    SELECT MIN(id_consumo)
    FROM consumos
    GROUP BY id_estudiante, id_producto, fecha_consumo
);

-- 3. From now on the database itself rejects duplicates.
CREATE UNIQUE INDEX IF NOT EXISTS ux_consumos_celda
    ON consumos (id_estudiante, id_producto, fecha_consumo);

-- 4. Rebuild the payments index (corrupt in production).
REINDEX idx_pagos_estudiante_fecha;

COMMIT;

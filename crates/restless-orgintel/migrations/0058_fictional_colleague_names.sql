-- Stable colleague names are allocated company-wide, including retired actors.
CREATE FUNCTION colleague_name(ordinal BIGINT) RETURNS TEXT
LANGUAGE SQL IMMUTABLE STRICT AS $$
  SELECT (ARRAY[
    'Alice', 'Bart', 'Coraline', 'Daria', 'Elsa', 'Frodo', 'Gandalf',
    'Hannah Montana', 'Ichabod Crane', 'Jessie', 'Katniss', 'Lilo',
    'Matilda', 'Nemo', 'Obi-Wan', 'Paddington', 'Quasimodo', 'Rapunzel',
    'Scooby-Doo', 'Totoro', 'Ursula', 'Velma', 'Winnie the Pooh',
    'Xena', 'Yoda', 'Zelda'
  ])[((ordinal - 1) % 26 + 1)::INTEGER]
  || CASE WHEN ordinal > 26 THEN ' ' || ((ordinal - 1) / 26 + 1)::TEXT ELSE '' END
$$;

CREATE TABLE colleague_name_allocator (
  singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
  allocated BIGINT NOT NULL CHECK (allocated >= 0)
);
INSERT INTO colleague_name_allocator (allocated) SELECT count(*) FROM actors WHERE kind='staff';

WITH ranked AS (
  SELECT id, display AS old_display, row_number() OVER (ORDER BY created_at,id) AS ordinal
  FROM actors WHERE kind='staff'
), renamed AS (
  UPDATE actors a SET display=colleague_name(r.ordinal)
  FROM ranked r WHERE a.id=r.id
  RETURNING a.id, r.old_display, a.display
)
INSERT INTO events (kind, actor_id, body)
SELECT 'actor_display_changed', id,
  jsonb_build_object('actor_id',id,'from_display',old_display,'to_display',display,
                    'reason','Alphabetical fictional colleague names')
FROM renamed WHERE old_display <> display;

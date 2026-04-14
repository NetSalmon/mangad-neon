CREATE TYPE tag_type AS ENUM (
    'genre',
    'artist',
    'origin',
    'serial',
    'character',
    'lang',
    'group'
    );

CREATE TABLE tags
(
    id           INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    type         tag_type NOT NULL,
    label        TEXT     NOT NULL,
    canonical_id INTEGER DEFAULT NULL ,
    FOREIGN KEY (canonical_id) REFERENCES tags (id) ON DELETE SET DEFAULT
);

CREATE UNIQUE INDEX tag_entities_type_label
    ON tags (type, label);

CREATE OR REPLACE FUNCTION fn_ensure_tag_canonical_integrity()
    RETURNS TRIGGER
AS
$$
DECLARE
    canonical_type      tag_type;
    target_id           INTEGER;
    target_canonical_id INTEGER;
BEGIN
    IF NEW.canonical_id IS NULL THEN
        NEW.canonical_id := NULL;
        RETURN NEW;
    END IF;

    SELECT type, id, canonical_id
    INTO STRICT canonical_type, target_id, target_canonical_id
    FROM tags
    WHERE id = NEW.canonical_id;

    IF target_canonical_id <> target_id THEN
        RAISE EXCEPTION 'Circular reference error: Target ID % is an alias, not a canonical tag.', NEW.canonical_id;
    END IF;

    IF canonical_type <> NEW.type THEN
        RAISE EXCEPTION 'Type mismatch: Tag type (%) must match canonical type (%).',
            NEW.type, canonical_type;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_tags_validate_canonical
    BEFORE INSERT OR UPDATE
    ON tags
    FOR EACH ROW
EXECUTE FUNCTION fn_ensure_tag_canonical_integrity();

CREATE TABLE metadata
(
    id         INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    page_count INTEGER                     NOT NULL,
    upload     TIMESTAMPTZ   DEFAULT now() NOT NULL,
    rating     NUMERIC(3, 1) DEFAULT 6.0   NOT NULL,
    CHECK ( id >= 0 ) ,
    CHECK ( rating BETWEEN 0.0 AND 10.0 )
);

CREATE TABLE literatures
(
    id          INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    metadata_id INTEGER NOT NULL,
    title       TEXT,
    description TEXT,
    FOREIGN KEY (metadata_id) REFERENCES metadata (id) ON DELETE CASCADE,
    lang        TEXT    NOT NULL DEFAULT 'en'
);

CREATE UNIQUE INDEX literatures_unq_idx_metadata_title ON literatures (metadata_id, title);

CREATE TABLE tag_metadata
(
    metadata_id INTEGER,
    tag_id      INTEGER,
    PRIMARY KEY (metadata_id, tag_id),
    FOREIGN KEY (metadata_id) REFERENCES metadata (id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags (id) ON DELETE CASCADE,
    weight      INTEGER DEFAULT 0 NOT NULL -- allow < 0
);

CREATE TYPE task_status AS ENUM (
    'success', 'processing', 'failure'
    );

CREATE TABLE tasks
(
    id            INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    status        task_status DEFAULT 'processing' NOT NULL,
    task          JSON                             NOT NULL,
    ending_reason TEXT,
    create_time   TIMESTAMPTZ DEFAULT now()        NOT NULL,
    update_time   TIMESTAMPTZ DEFAULT now()        NOT NULL
);

CREATE OR REPLACE FUNCTION fn_auto_modify_update_time()
    RETURNS TRIGGER
AS
$$
BEGIN
    NEW.update_time = now();
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_auto_modify_update_time
    BEFORE UPDATE
    ON tasks
    FOR EACH ROW
EXECUTE FUNCTION fn_auto_modify_update_time();

CREATE TABLE tokens
(
    id          uuid PRIMARY KEY,
    hash        TEXT                      NOT NULL,
    remark      TEXT,
    description TEXT,
    create_time TIMESTAMPTZ DEFAULT now() NOT NULL,
    revoke_time TIMESTAMPTZ,
    expire_time TIMESTAMPTZ DEFAULT now() + interval '30 day',
    is_revoked  BOOLEAN     DEFAULT false NOT NULL
);

CREATE OR REPLACE FUNCTION fn_auto_modify_revoke_time()
    RETURNS TRIGGER
AS
$$
BEGIN
    IF NEW.is_revoked = true AND OLD.is_revoked = false THEN
        NEW.revoke_time = now();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_auto_modify_revoke_time
    BEFORE UPDATE
    ON tokens
    FOR EACH ROW
EXECUTE FUNCTION fn_auto_modify_revoke_time();

CREATE OR REPLACE FUNCTION fn_literatures_modify_notice()
    RETURNS TRIGGER
AS
$$
BEGIN
    PERFORM pg_notify('literatures', CAST(NEW.id AS TEXT));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_literatures_modify_notice
    AFTER INSERT OR UPDATE OR DELETE
    ON literatures
    FOR EACH ROW
EXECUTE FUNCTION fn_literatures_modify_notice();

CREATE OR REPLACE FUNCTION fn_tags_modify_notice()
    RETURNS TRIGGER
AS
$$
BEGIN
    PERFORM pg_notify('tags', CAST(NEW.id AS TEXT));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_tags_modify_notice
    AFTER INSERT OR UPDATE OR DELETE
    ON tags
    FOR EACH ROW
EXECUTE FUNCTION fn_tags_modify_notice();

CREATE OR REPLACE FUNCTION fn_tag_metadata_modify_notice()
    RETURNS TRIGGER
AS
$$
BEGIN
    PERFORM pg_notify('tag_metadata', CAST(NEW.metadata_id AS TEXT));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_tag_metadata_modify_notice
    AFTER INSERT OR UPDATE OR DELETE
    ON tag_metadata
    FOR EACH ROW
EXECUTE FUNCTION fn_tag_metadata_modify_notice();

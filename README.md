# DATABASE DEFINE
```postgresql
CREATE TYPE tag_type AS ENUM (
    'genre',
    'artist',
    'origin',
    'serial',
    'chara',
    'lang',
    'group'
);

CREATE TABLE TAGS (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY ,
    type tag_type NOT NULL ,
    label TEXT NOT NULL ,
    canonical_id INTEGER ,
    ref_count INTEGER DEFAULT 0 ,
    FOREIGN KEY (canonical_id) REFERENCES TAGS(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX tag_entities_type_data
ON TAGS ( type, label);

CREATE OR REPLACE FUNCTION fn_ensure_tag_canonical_integrity()
RETURNS TRIGGER
AS $$
DECLARE
    canonical_type tag_type;
    target_id INTEGER;
    target_canonical_id INTEGER;
BEGIN
    IF NEW.canonical_id IS NULL THEN
        NEW.canonical_id := NEW.id; -- this segment is ok
        RETURN NEW;
    END IF;

    SELECT type, id, canonical_id INTO STRICT canonical_type, target_id, target_canonical_id
    FROM TAGS
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
BEFORE INSERT OR UPDATE ON TAGS
FOR EACH ROW EXECUTE FUNCTION fn_ensure_tag_canonical_integrity();

CREATE TABLE METADATA (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY ,
    page_count INTEGER NOT NULL ,
    upload TIMESTAMP DEFAULT now()
);

CREATE TABLE LITERATURES (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY ,
    metadata_id INTEGER ,
    title TEXT ,
    description TEXT ,
    FOREIGN KEY (metadata_id) REFERENCES METADATA(id) ON DELETE CASCADE ,
    lang TEXT DEFAULT 'en'
);

CREATE UNIQUE INDEX literatures_unq_idx_metadata_title ON LITERATURES (metadata_id, title);

CREATE OR REPLACE FUNCTION fn_tag_ref_counter()
    RETURNS TRIGGER AS $$
BEGIN
    IF (TG_OP = 'INSERT') THEN
        UPDATE TAGS SET ref_count = ref_count + 1 WHERE id = NEW.tag_id;
        RETURN NEW;
    ELSIF (TG_OP = 'UPDATE') THEN
        IF OLD.tag_id <> NEW.tag_id THEN
            UPDATE TAGS SET ref_count = ref_count - 1 WHERE id = OLD.tag_id;
            UPDATE TAGS SET ref_count = ref_count + 1 WHERE id = NEW.tag_id;
        END IF;
        RETURN NEW;
    ELSIF (TG_OP = 'DELETE') THEN
        UPDATE TAGS SET ref_count = ref_count - 1 WHERE id = OLD.tag_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE TAG_METADATA (
    metadata_id INTEGER ,
    tag_id INTEGER ,
    PRIMARY KEY (metadata_id, tag_id) ,
    FOREIGN KEY (metadata_id) REFERENCES METADATA (id) ON DELETE CASCADE ,
    FOREIGN KEY (tag_id) REFERENCES TAGS (id) ON DELETE CASCADE
);

CREATE TRIGGER trg_tag_ref_count
BEFORE INSERT OR UPDATE OR DELETE ON TAG_METADATA
FOR EACH ROW EXECUTE FUNCTION fn_tag_ref_counter();

CREATE TYPE task_status AS ENUM (
    'success', 'processing', 'failure'
    );

CREATE TABLE TASKS (
    id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY ,
    status task_status DEFAULT 'processing' ,
    task JSON NOT NULL
);
```
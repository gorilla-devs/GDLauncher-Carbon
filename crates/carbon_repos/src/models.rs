//! Generated Rust types from SQLite schema
//! Do not edit manually

use rusqlite;

#[derive(Debug, Clone)]
pub struct PrismaMigrations {
    /// Primary key. Indexed
    pub id: String,
    pub checksum: String,
    pub finished_at: Option<String>,
    pub migration_name: String,
    pub logs: Option<String>,
    pub rolled_back_at: Option<String>,
    pub started_at: String,
    pub applied_steps_count: u64,
}

impl PrismaMigrations {
    pub const TABLE: &'static str = "_prisma_migrations";
    pub const ID: &'static str = "id";
    pub const CHECKSUM: &'static str = "checksum";
    pub const FINISHED_AT: &'static str = "finished_at";
    pub const MIGRATION_NAME: &'static str = "migration_name";
    pub const LOGS: &'static str = "logs";
    pub const ROLLED_BACK_AT: &'static str = "rolled_back_at";
    pub const STARTED_AT: &'static str = "started_at";
    pub const APPLIED_STEPS_COUNT: &'static str = "applied_steps_count";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        checksum: String,
        finished_at: Option<String>,
        migration_name: String,
        logs: Option<String>,
        rolled_back_at: Option<String>,
        started_at: String,
        applied_steps_count: u64,
    ) -> PrismaMigrations {
        PrismaMigrations {
            id,
            checksum,
            finished_at,
            migration_name,
            logs,
            rolled_back_at,
            started_at,
            applied_steps_count,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            checksum: row.get(Self::CHECKSUM)?,
            finished_at: row.get(Self::FINISHED_AT)?,
            migration_name: row.get(Self::MIGRATION_NAME)?,
            logs: row.get(Self::LOGS)?,
            rolled_back_at: row.get(Self::ROLLED_BACK_AT)?,
            started_at: row.get(Self::STARTED_AT)?,
            applied_steps_count: row.get(Self::APPLIED_STEPS_COUNT)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::ID,
            Self::CHECKSUM,
            Self::FINISHED_AT,
            Self::MIGRATION_NAME,
            Self::LOGS,
            Self::ROLLED_BACK_AT,
            Self::STARTED_AT,
            Self::APPLIED_STEPS_COUNT,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Java {
    /// Primary key. Indexed
    pub id: String,
    /// Indexed
    pub path: String,
    pub major: i64,
    pub full_version: String,
    pub _type: String,
    pub os: String,
    pub arch: String,
    pub vendor: String,
    pub is_valid: bool,
}

impl Java {
    pub const TABLE: &'static str = "Java";
    pub const ID: &'static str = "id";
    pub const PATH: &'static str = "path";
    pub const MAJOR: &'static str = "major";
    pub const FULLVERSION: &'static str = "fullVersion";
    pub const TYPE: &'static str = "type";
    pub const OS: &'static str = "os";
    pub const ARCH: &'static str = "arch";
    pub const VENDOR: &'static str = "vendor";
    pub const ISVALID: &'static str = "isValid";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        path: String,
        major: i64,
        full_version: String,
        _type: String,
        os: String,
        arch: String,
        vendor: String,
        is_valid: bool,
    ) -> Java {
        Java {
            id,
            path,
            major,
            full_version,
            _type,
            os,
            arch,
            vendor,
            is_valid,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            path: row.get(Self::PATH)?,
            major: row.get(Self::MAJOR)?,
            full_version: row.get(Self::FULLVERSION)?,
            _type: row.get(Self::TYPE)?,
            os: row.get(Self::OS)?,
            arch: row.get(Self::ARCH)?,
            vendor: row.get(Self::VENDOR)?,
            is_valid: row.get(Self::ISVALID)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::ID,
            Self::PATH,
            Self::MAJOR,
            Self::FULLVERSION,
            Self::TYPE,
            Self::OS,
            Self::ARCH,
            Self::VENDOR,
            Self::ISVALID,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Account {
    /// Primary key. Indexed
    pub uuid: String,
    pub username: String,
    pub access_token: Option<String>,
    pub token_expires: Option<String>,
    pub ms_refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub last_used: String,
    pub skin_id: Option<String>,
}

impl Account {
    pub const TABLE: &'static str = "Account";
    pub const UUID: &'static str = "uuid";
    pub const USERNAME: &'static str = "username";
    pub const ACCESSTOKEN: &'static str = "accessToken";
    pub const TOKENEXPIRES: &'static str = "tokenExpires";
    pub const MSREFRESHTOKEN: &'static str = "msRefreshToken";
    pub const IDTOKEN: &'static str = "idToken";
    pub const LASTUSED: &'static str = "lastUsed";
    pub const SKINID: &'static str = "skinId";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        uuid: String,
        username: String,
        access_token: Option<String>,
        token_expires: Option<String>,
        ms_refresh_token: Option<String>,
        id_token: Option<String>,
        last_used: String,
        skin_id: Option<String>,
    ) -> Account {
        Account {
            uuid,
            username,
            access_token,
            token_expires,
            ms_refresh_token,
            id_token,
            last_used,
            skin_id,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            uuid: row.get(Self::UUID)?,
            username: row.get(Self::USERNAME)?,
            access_token: row.get(Self::ACCESSTOKEN)?,
            token_expires: row.get(Self::TOKENEXPIRES)?,
            ms_refresh_token: row.get(Self::MSREFRESHTOKEN)?,
            id_token: row.get(Self::IDTOKEN)?,
            last_used: row.get(Self::LASTUSED)?,
            skin_id: row.get(Self::SKINID)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::UUID,
            Self::USERNAME,
            Self::ACCESSTOKEN,
            Self::TOKENEXPIRES,
            Self::MSREFRESHTOKEN,
            Self::IDTOKEN,
            Self::LASTUSED,
            Self::SKINID,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::UUID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Skin {
    /// Primary key. Indexed
    pub id: String,
    pub skin: Vec<u8>,
}

impl Skin {
    pub const TABLE: &'static str = "Skin";
    pub const ID: &'static str = "id";
    pub const SKIN: &'static str = "skin";

    #[allow(clippy::too_many_arguments)]
    pub fn new(id: String, skin: Vec<u8>) -> Skin {
        Skin { id, skin }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            skin: row.get(Self::SKIN)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::ID, Self::SKIN]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Httpcache {
    /// Primary key. Indexed
    pub url: String,
    pub status_code: i64,
    pub data: Vec<u8>,
    pub expires_at: Option<String>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

impl Httpcache {
    pub const TABLE: &'static str = "HTTPCache";
    pub const URL: &'static str = "url";
    pub const STATUS_CODE: &'static str = "status_code";
    pub const DATA: &'static str = "data";
    pub const EXPIRESAT: &'static str = "expiresAt";
    pub const LASTMODIFIED: &'static str = "lastModified";
    pub const ETAG: &'static str = "etag";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        url: String,
        status_code: i64,
        data: Vec<u8>,
        expires_at: Option<String>,
        last_modified: Option<String>,
        etag: Option<String>,
    ) -> Httpcache {
        Httpcache {
            url,
            status_code,
            data,
            expires_at,
            last_modified,
            etag,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            url: row.get(Self::URL)?,
            status_code: row.get(Self::STATUS_CODE)?,
            data: row.get(Self::DATA)?,
            expires_at: row.get(Self::EXPIRESAT)?,
            last_modified: row.get(Self::LASTMODIFIED)?,
            etag: row.get(Self::ETAG)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::URL,
            Self::STATUS_CODE,
            Self::DATA,
            Self::EXPIRESAT,
            Self::LASTMODIFIED,
            Self::ETAG,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::URL
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Activedownloads {
    /// Primary key. Indexed
    pub url: String,
    /// Indexed
    pub file_id: String,
}

impl Activedownloads {
    pub const TABLE: &'static str = "ActiveDownloads";
    pub const URL: &'static str = "url";
    pub const FILE_ID: &'static str = "file_id";

    #[allow(clippy::too_many_arguments)]
    pub fn new(url: String, file_id: String) -> Activedownloads {
        Activedownloads { url, file_id }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            url: row.get(Self::URL)?,
            file_id: row.get(Self::FILE_ID)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::URL, Self::FILE_ID]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::URL
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Instancegroup {
    /// Primary key
    pub id: i64,
    pub name: String,
    pub group_index: i64,
}

impl Instancegroup {
    pub const TABLE: &'static str = "InstanceGroup";
    pub const ID: &'static str = "id";
    pub const NAME: &'static str = "name";
    pub const GROUPINDEX: &'static str = "groupIndex";

    #[allow(clippy::too_many_arguments)]
    pub fn new(id: i64, name: String, group_index: i64) -> Instancegroup {
        Instancegroup {
            id,
            name,
            group_index,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            name: row.get(Self::NAME)?,
            group_index: row.get(Self::GROUPINDEX)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::ID, Self::NAME, Self::GROUPINDEX]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: i64) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Curseforgemodcache {
    /// Primary key. Foreign key to ModMetadata.id. Indexed
    pub metadata_id: String,
    pub murmur2: i64,
    /// Indexed
    pub project_id: i64,
    /// Indexed
    pub file_id: i64,
    pub name: String,
    pub version: String,
    pub urlslug: String,
    pub summary: String,
    pub authors: String,
    pub release_type: i64,
    pub update_paths: String,
    pub cached_at: String,
}

impl Curseforgemodcache {
    pub const TABLE: &'static str = "CurseForgeModCache";
    pub const METADATAID: &'static str = "metadataId";
    pub const MURMUR2: &'static str = "murmur2";
    pub const PROJECTID: &'static str = "projectId";
    pub const FILEID: &'static str = "fileId";
    pub const NAME: &'static str = "name";
    pub const VERSION: &'static str = "version";
    pub const URLSLUG: &'static str = "urlslug";
    pub const SUMMARY: &'static str = "summary";
    pub const AUTHORS: &'static str = "authors";
    pub const RELEASETYPE: &'static str = "releaseType";
    pub const UPDATEPATHS: &'static str = "updatePaths";
    pub const CACHEDAT: &'static str = "cachedAt";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        metadata_id: String,
        murmur2: i64,
        project_id: i64,
        file_id: i64,
        name: String,
        version: String,
        urlslug: String,
        summary: String,
        authors: String,
        release_type: i64,
        update_paths: String,
        cached_at: String,
    ) -> Curseforgemodcache {
        Curseforgemodcache {
            metadata_id,
            murmur2,
            project_id,
            file_id,
            name,
            version,
            urlslug,
            summary,
            authors,
            release_type,
            update_paths,
            cached_at,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            metadata_id: row.get(Self::METADATAID)?,
            murmur2: row.get(Self::MURMUR2)?,
            project_id: row.get(Self::PROJECTID)?,
            file_id: row.get(Self::FILEID)?,
            name: row.get(Self::NAME)?,
            version: row.get(Self::VERSION)?,
            urlslug: row.get(Self::URLSLUG)?,
            summary: row.get(Self::SUMMARY)?,
            authors: row.get(Self::AUTHORS)?,
            release_type: row.get(Self::RELEASETYPE)?,
            update_paths: row.get(Self::UPDATEPATHS)?,
            cached_at: row.get(Self::CACHEDAT)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::METADATAID,
            Self::MURMUR2,
            Self::PROJECTID,
            Self::FILEID,
            Self::NAME,
            Self::VERSION,
            Self::URLSLUG,
            Self::SUMMARY,
            Self::AUTHORS,
            Self::RELEASETYPE,
            Self::UPDATEPATHS,
            Self::CACHEDAT,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::METADATAID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Modrinthmodcache {
    /// Primary key. Foreign key to ModMetadata.id. Indexed
    pub metadata_id: String,
    pub sha512: String,
    /// Indexed
    pub project_id: String,
    /// Indexed
    pub version_id: String,
    pub title: String,
    pub version: String,
    pub urlslug: String,
    pub description: String,
    pub authors: String,
    pub release_type: i64,
    pub update_paths: String,
    pub filename: String,
    pub file_url: String,
    pub cached_at: String,
}

impl Modrinthmodcache {
    pub const TABLE: &'static str = "ModrinthModCache";
    pub const METADATAID: &'static str = "metadataId";
    pub const SHA512: &'static str = "sha512";
    pub const PROJECTID: &'static str = "projectId";
    pub const VERSIONID: &'static str = "versionId";
    pub const TITLE: &'static str = "title";
    pub const VERSION: &'static str = "version";
    pub const URLSLUG: &'static str = "urlslug";
    pub const DESCRIPTION: &'static str = "description";
    pub const AUTHORS: &'static str = "authors";
    pub const RELEASETYPE: &'static str = "releaseType";
    pub const UPDATEPATHS: &'static str = "updatePaths";
    pub const FILENAME: &'static str = "filename";
    pub const FILEURL: &'static str = "fileUrl";
    pub const CACHEDAT: &'static str = "cachedAt";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        metadata_id: String,
        sha512: String,
        project_id: String,
        version_id: String,
        title: String,
        version: String,
        urlslug: String,
        description: String,
        authors: String,
        release_type: i64,
        update_paths: String,
        filename: String,
        file_url: String,
        cached_at: String,
    ) -> Modrinthmodcache {
        Modrinthmodcache {
            metadata_id,
            sha512,
            project_id,
            version_id,
            title,
            version,
            urlslug,
            description,
            authors,
            release_type,
            update_paths,
            filename,
            file_url,
            cached_at,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            metadata_id: row.get(Self::METADATAID)?,
            sha512: row.get(Self::SHA512)?,
            project_id: row.get(Self::PROJECTID)?,
            version_id: row.get(Self::VERSIONID)?,
            title: row.get(Self::TITLE)?,
            version: row.get(Self::VERSION)?,
            urlslug: row.get(Self::URLSLUG)?,
            description: row.get(Self::DESCRIPTION)?,
            authors: row.get(Self::AUTHORS)?,
            release_type: row.get(Self::RELEASETYPE)?,
            update_paths: row.get(Self::UPDATEPATHS)?,
            filename: row.get(Self::FILENAME)?,
            file_url: row.get(Self::FILEURL)?,
            cached_at: row.get(Self::CACHEDAT)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::METADATAID,
            Self::SHA512,
            Self::PROJECTID,
            Self::VERSIONID,
            Self::TITLE,
            Self::VERSION,
            Self::URLSLUG,
            Self::DESCRIPTION,
            Self::AUTHORS,
            Self::RELEASETYPE,
            Self::UPDATEPATHS,
            Self::FILENAME,
            Self::FILEURL,
            Self::CACHEDAT,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::METADATAID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Localmodimagecache {
    /// Primary key. Foreign key to ModMetadata.id. Indexed
    pub metadata_id: String,
    pub data: Vec<u8>,
}

impl Localmodimagecache {
    pub const TABLE: &'static str = "LocalModImageCache";
    pub const METADATAID: &'static str = "metadataId";
    pub const DATA: &'static str = "data";

    #[allow(clippy::too_many_arguments)]
    pub fn new(metadata_id: String, data: Vec<u8>) -> Localmodimagecache {
        Localmodimagecache { metadata_id, data }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            metadata_id: row.get(Self::METADATAID)?,
            data: row.get(Self::DATA)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::METADATAID, Self::DATA]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::METADATAID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Curseforgemodimagecache {
    /// Primary key. Foreign key to CurseForgeModCache.metadataId. Indexed
    pub metadata_id: String,
    pub url: String,
    pub data: Option<Vec<u8>>,
    pub up_to_date: i64,
}

impl Curseforgemodimagecache {
    pub const TABLE: &'static str = "CurseForgeModImageCache";
    pub const METADATAID: &'static str = "metadataId";
    pub const URL: &'static str = "url";
    pub const DATA: &'static str = "data";
    pub const UPTODATE: &'static str = "upToDate";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        metadata_id: String,
        url: String,
        data: Option<Vec<u8>>,
        up_to_date: i64,
    ) -> Curseforgemodimagecache {
        Curseforgemodimagecache {
            metadata_id,
            url,
            data,
            up_to_date,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            metadata_id: row.get(Self::METADATAID)?,
            url: row.get(Self::URL)?,
            data: row.get(Self::DATA)?,
            up_to_date: row.get(Self::UPTODATE)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::METADATAID, Self::URL, Self::DATA, Self::UPTODATE]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::METADATAID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Modrinthmodimagecache {
    /// Primary key. Foreign key to ModrinthModCache.metadataId. Indexed
    pub metadata_id: String,
    pub url: String,
    pub data: Option<Vec<u8>>,
    pub up_to_date: i64,
}

impl Modrinthmodimagecache {
    pub const TABLE: &'static str = "ModrinthModImageCache";
    pub const METADATAID: &'static str = "metadataId";
    pub const URL: &'static str = "url";
    pub const DATA: &'static str = "data";
    pub const UPTODATE: &'static str = "upToDate";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        metadata_id: String,
        url: String,
        data: Option<Vec<u8>>,
        up_to_date: i64,
    ) -> Modrinthmodimagecache {
        Modrinthmodimagecache {
            metadata_id,
            url,
            data,
            up_to_date,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            metadata_id: row.get(Self::METADATAID)?,
            url: row.get(Self::URL)?,
            data: row.get(Self::DATA)?,
            up_to_date: row.get(Self::UPTODATE)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::METADATAID, Self::URL, Self::DATA, Self::UPTODATE]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::METADATAID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Curseforgemodpackcache {
    /// Primary key. Indexed
    pub project_id: i64,
    /// Indexed
    pub file_id: i64,
    pub modpack_name: String,
    pub version_name: String,
    pub url_slug: String,
    pub updated_at: String,
}

impl Curseforgemodpackcache {
    pub const TABLE: &'static str = "CurseForgeModpackCache";
    pub const PROJECTID: &'static str = "projectId";
    pub const FILEID: &'static str = "fileId";
    pub const MODPACKNAME: &'static str = "modpackName";
    pub const VERSIONNAME: &'static str = "versionName";
    pub const URLSLUG: &'static str = "urlSlug";
    pub const UPDATEDAT: &'static str = "updatedAt";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: i64,
        file_id: i64,
        modpack_name: String,
        version_name: String,
        url_slug: String,
        updated_at: String,
    ) -> Curseforgemodpackcache {
        Curseforgemodpackcache {
            project_id,
            file_id,
            modpack_name,
            version_name,
            url_slug,
            updated_at,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            project_id: row.get(Self::PROJECTID)?,
            file_id: row.get(Self::FILEID)?,
            modpack_name: row.get(Self::MODPACKNAME)?,
            version_name: row.get(Self::VERSIONNAME)?,
            url_slug: row.get(Self::URLSLUG)?,
            updated_at: row.get(Self::UPDATEDAT)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::PROJECTID,
            Self::FILEID,
            Self::MODPACKNAME,
            Self::VERSIONNAME,
            Self::URLSLUG,
            Self::UPDATEDAT,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: i64) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::PROJECTID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Modrinthmodpackcache {
    /// Primary key. Indexed
    pub project_id: String,
    /// Indexed
    pub version_id: String,
    pub modpack_name: String,
    pub version_name: String,
    pub url_slug: String,
    pub updated_at: String,
}

impl Modrinthmodpackcache {
    pub const TABLE: &'static str = "ModrinthModpackCache";
    pub const PROJECTID: &'static str = "projectId";
    pub const VERSIONID: &'static str = "versionId";
    pub const MODPACKNAME: &'static str = "modpackName";
    pub const VERSIONNAME: &'static str = "versionName";
    pub const URLSLUG: &'static str = "urlSlug";
    pub const UPDATEDAT: &'static str = "updatedAt";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: String,
        version_id: String,
        modpack_name: String,
        version_name: String,
        url_slug: String,
        updated_at: String,
    ) -> Modrinthmodpackcache {
        Modrinthmodpackcache {
            project_id,
            version_id,
            modpack_name,
            version_name,
            url_slug,
            updated_at,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            project_id: row.get(Self::PROJECTID)?,
            version_id: row.get(Self::VERSIONID)?,
            modpack_name: row.get(Self::MODPACKNAME)?,
            version_name: row.get(Self::VERSIONNAME)?,
            url_slug: row.get(Self::URLSLUG)?,
            updated_at: row.get(Self::UPDATEDAT)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::PROJECTID,
            Self::VERSIONID,
            Self::MODPACKNAME,
            Self::VERSIONNAME,
            Self::URLSLUG,
            Self::UPDATEDAT,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::PROJECTID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Curseforgemodpackimagecache {
    /// Primary key. Foreign key to CurseForgeModpackCache.projectId. Indexed
    pub project_id: i64,
    /// Foreign key to CurseForgeModpackCache.fileId. Indexed
    pub file_id: i64,
    pub url: String,
    pub data: Option<Vec<u8>>,
}

impl Curseforgemodpackimagecache {
    pub const TABLE: &'static str = "CurseForgeModpackImageCache";
    pub const PROJECTID: &'static str = "projectId";
    pub const FILEID: &'static str = "fileId";
    pub const URL: &'static str = "url";
    pub const DATA: &'static str = "data";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: i64,
        file_id: i64,
        url: String,
        data: Option<Vec<u8>>,
    ) -> Curseforgemodpackimagecache {
        Curseforgemodpackimagecache {
            project_id,
            file_id,
            url,
            data,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            project_id: row.get(Self::PROJECTID)?,
            file_id: row.get(Self::FILEID)?,
            url: row.get(Self::URL)?,
            data: row.get(Self::DATA)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::PROJECTID, Self::FILEID, Self::URL, Self::DATA]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: i64) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::PROJECTID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Modrinthmodpackimagecache {
    /// Primary key. Foreign key to ModrinthModpackCache.projectId. Indexed
    pub project_id: String,
    /// Foreign key to ModrinthModpackCache.versionId. Indexed
    pub version_id: String,
    pub url: String,
    pub data: Option<Vec<u8>>,
}

impl Modrinthmodpackimagecache {
    pub const TABLE: &'static str = "ModrinthModpackImageCache";
    pub const PROJECTID: &'static str = "projectId";
    pub const VERSIONID: &'static str = "versionId";
    pub const URL: &'static str = "url";
    pub const DATA: &'static str = "data";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: String,
        version_id: String,
        url: String,
        data: Option<Vec<u8>>,
    ) -> Modrinthmodpackimagecache {
        Modrinthmodpackimagecache {
            project_id,
            version_id,
            url,
            data,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            project_id: row.get(Self::PROJECTID)?,
            version_id: row.get(Self::VERSIONID)?,
            url: row.get(Self::URL)?,
            data: row.get(Self::DATA)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::PROJECTID, Self::VERSIONID, Self::URL, Self::DATA]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::PROJECTID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Javaprofile {
    /// Primary key. Indexed
    pub name: String,
    pub is_system_profile: bool,
    /// Foreign key to Java.id
    pub java_id: Option<String>,
}

impl Javaprofile {
    pub const TABLE: &'static str = "JavaProfile";
    pub const NAME: &'static str = "name";
    pub const ISSYSTEMPROFILE: &'static str = "isSystemProfile";
    pub const JAVAID: &'static str = "javaId";

    #[allow(clippy::too_many_arguments)]
    pub fn new(name: String, is_system_profile: bool, java_id: Option<String>) -> Javaprofile {
        Javaprofile {
            name,
            is_system_profile,
            java_id,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            name: row.get(Self::NAME)?,
            is_system_profile: row.get(Self::ISSYSTEMPROFILE)?,
            java_id: row.get(Self::JAVAID)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::NAME, Self::ISSYSTEMPROFILE, Self::JAVAID]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::NAME
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Instance {
    /// Primary key
    pub id: i64,
    pub name: String,
    /// Indexed
    pub shortpath: String,
    pub favorite: bool,
    pub has_pack_update: bool,
    pub index: i64,
    /// Foreign key to InstanceGroup.id
    pub group_id: i64,
}

impl Instance {
    pub const TABLE: &'static str = "Instance";
    pub const ID: &'static str = "id";
    pub const NAME: &'static str = "name";
    pub const SHORTPATH: &'static str = "shortpath";
    pub const FAVORITE: &'static str = "favorite";
    pub const HASPACKUPDATE: &'static str = "hasPackUpdate";
    pub const INDEX: &'static str = "index";
    pub const GROUPID: &'static str = "groupId";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        name: String,
        shortpath: String,
        favorite: bool,
        has_pack_update: bool,
        index: i64,
        group_id: i64,
    ) -> Instance {
        Instance {
            id,
            name,
            shortpath,
            favorite,
            has_pack_update,
            index,
            group_id,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            name: row.get(Self::NAME)?,
            shortpath: row.get(Self::SHORTPATH)?,
            favorite: row.get(Self::FAVORITE)?,
            has_pack_update: row.get(Self::HASPACKUPDATE)?,
            index: row.get(Self::INDEX)?,
            group_id: row.get(Self::GROUPID)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::ID,
            Self::NAME,
            Self::SHORTPATH,
            Self::FAVORITE,
            Self::HASPACKUPDATE,
            Self::INDEX,
            Self::GROUPID,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: i64) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Modfilecache {
    /// Primary key. Indexed
    pub id: String,
    pub last_updated_at: String,
    /// Foreign key to Instance.id. Indexed
    pub instance_id: i64,
    /// Indexed
    pub filename: String,
    pub filesize: i64,
    pub enabled: bool,
    /// Foreign key to ModMetadata.id
    pub metadata_id: String,
}

impl Modfilecache {
    pub const TABLE: &'static str = "ModFileCache";
    pub const ID: &'static str = "id";
    pub const LASTUPDATEDAT: &'static str = "lastUpdatedAt";
    pub const INSTANCEID: &'static str = "instanceId";
    pub const FILENAME: &'static str = "filename";
    pub const FILESIZE: &'static str = "filesize";
    pub const ENABLED: &'static str = "enabled";
    pub const METADATAID: &'static str = "metadataId";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        last_updated_at: String,
        instance_id: i64,
        filename: String,
        filesize: i64,
        enabled: bool,
        metadata_id: String,
    ) -> Modfilecache {
        Modfilecache {
            id,
            last_updated_at,
            instance_id,
            filename,
            filesize,
            enabled,
            metadata_id,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            last_updated_at: row.get(Self::LASTUPDATEDAT)?,
            instance_id: row.get(Self::INSTANCEID)?,
            filename: row.get(Self::FILENAME)?,
            filesize: row.get(Self::FILESIZE)?,
            enabled: row.get(Self::ENABLED)?,
            metadata_id: row.get(Self::METADATAID)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::ID,
            Self::LASTUPDATEDAT,
            Self::INSTANCEID,
            Self::FILENAME,
            Self::FILESIZE,
            Self::ENABLED,
            Self::METADATAID,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Versioninfocache {
    /// Primary key. Indexed
    pub id: String,
    pub last_updated_at: String,
    pub version_info: Vec<u8>,
}

impl Versioninfocache {
    pub const TABLE: &'static str = "VersionInfoCache";
    pub const ID: &'static str = "id";
    pub const LASTUPDATEDAT: &'static str = "lastUpdatedAt";
    pub const VERSIONINFO: &'static str = "versionInfo";

    #[allow(clippy::too_many_arguments)]
    pub fn new(id: String, last_updated_at: String, version_info: Vec<u8>) -> Versioninfocache {
        Versioninfocache {
            id,
            last_updated_at,
            version_info,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            last_updated_at: row.get(Self::LASTUPDATEDAT)?,
            version_info: row.get(Self::VERSIONINFO)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::ID, Self::LASTUPDATEDAT, Self::VERSIONINFO]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Lwjglmetacache {
    /// Primary key. Indexed
    pub id: String,
    pub last_updated_at: String,
    pub lwjgl: Vec<u8>,
}

impl Lwjglmetacache {
    pub const TABLE: &'static str = "LwjglMetaCache";
    pub const ID: &'static str = "id";
    pub const LASTUPDATEDAT: &'static str = "lastUpdatedAt";
    pub const LWJGL: &'static str = "lwjgl";

    #[allow(clippy::too_many_arguments)]
    pub fn new(id: String, last_updated_at: String, lwjgl: Vec<u8>) -> Lwjglmetacache {
        Lwjglmetacache {
            id,
            last_updated_at,
            lwjgl,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            last_updated_at: row.get(Self::LASTUPDATEDAT)?,
            lwjgl: row.get(Self::LWJGL)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::ID, Self::LASTUPDATEDAT, Self::LWJGL]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Modmetadata {
    /// Primary key. Indexed
    pub id: String,
    pub last_updated_at: String,
    pub murmur2: i64,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub name: Option<String>,
    pub modid: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: String,
}

impl Modmetadata {
    pub const TABLE: &'static str = "ModMetadata";
    pub const ID: &'static str = "id";
    pub const LASTUPDATEDAT: &'static str = "lastUpdatedAt";
    pub const MURMUR2: &'static str = "murmur2";
    pub const SHA512: &'static str = "sha512";
    pub const SHA1: &'static str = "sha1";
    pub const NAME: &'static str = "name";
    pub const MODID: &'static str = "modid";
    pub const VERSION: &'static str = "version";
    pub const DESCRIPTION: &'static str = "description";
    pub const AUTHORS: &'static str = "authors";
    pub const MODLOADERS: &'static str = "modloaders";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        last_updated_at: String,
        murmur2: i64,
        sha512: Vec<u8>,
        sha1: Vec<u8>,
        name: Option<String>,
        modid: Option<String>,
        version: Option<String>,
        description: Option<String>,
        authors: Option<String>,
        modloaders: String,
    ) -> Modmetadata {
        Modmetadata {
            id,
            last_updated_at,
            murmur2,
            sha512,
            sha1,
            name,
            modid,
            version,
            description,
            authors,
            modloaders,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            last_updated_at: row.get(Self::LASTUPDATEDAT)?,
            murmur2: row.get(Self::MURMUR2)?,
            sha512: row.get(Self::SHA512)?,
            sha1: row.get(Self::SHA1)?,
            name: row.get(Self::NAME)?,
            modid: row.get(Self::MODID)?,
            version: row.get(Self::VERSION)?,
            description: row.get(Self::DESCRIPTION)?,
            authors: row.get(Self::AUTHORS)?,
            modloaders: row.get(Self::MODLOADERS)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::ID,
            Self::LASTUPDATEDAT,
            Self::MURMUR2,
            Self::SHA512,
            Self::SHA1,
            Self::NAME,
            Self::MODID,
            Self::VERSION,
            Self::DESCRIPTION,
            Self::AUTHORS,
            Self::MODLOADERS,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Assetsmetacache {
    /// Primary key. Indexed
    pub id: String,
    pub last_updated_at: String,
    pub assets_index: Vec<u8>,
}

impl Assetsmetacache {
    pub const TABLE: &'static str = "AssetsMetaCache";
    pub const ID: &'static str = "id";
    pub const LASTUPDATEDAT: &'static str = "lastUpdatedAt";
    pub const ASSETSINDEX: &'static str = "assetsIndex";

    #[allow(clippy::too_many_arguments)]
    pub fn new(id: String, last_updated_at: String, assets_index: Vec<u8>) -> Assetsmetacache {
        Assetsmetacache {
            id,
            last_updated_at,
            assets_index,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            last_updated_at: row.get(Self::LASTUPDATEDAT)?,
            assets_index: row.get(Self::ASSETSINDEX)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::ID, Self::LASTUPDATEDAT, Self::ASSETSINDEX]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Partialversioninfocache {
    /// Primary key. Indexed
    pub id: String,
    pub last_updated_at: String,
    pub partial_version_info: Vec<u8>,
}

impl Partialversioninfocache {
    pub const TABLE: &'static str = "PartialVersionInfoCache";
    pub const ID: &'static str = "id";
    pub const LASTUPDATEDAT: &'static str = "lastUpdatedAt";
    pub const PARTIALVERSIONINFO: &'static str = "partialVersionInfo";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        last_updated_at: String,
        partial_version_info: Vec<u8>,
    ) -> Partialversioninfocache {
        Partialversioninfocache {
            id,
            last_updated_at,
            partial_version_info,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            last_updated_at: row.get(Self::LASTUPDATEDAT)?,
            partial_version_info: row.get(Self::PARTIALVERSIONINFO)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[Self::ID, Self::LASTUPDATEDAT, Self::PARTIALVERSIONINFO]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: String) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

#[derive(Debug, Clone)]
pub struct Appconfiguration {
    /// Primary key. Indexed
    pub id: i64,
    pub theme: String,
    pub reduced_motion: bool,
    pub language: String,
    pub discord_integration: bool,
    pub release_channel: String,
    pub last_app_version: Option<String>,
    /// Foreign key to Account.uuid
    pub active_account_uuid: Option<String>,
    pub concurrent_downloads: i64,
    pub download_dependencies: bool,
    pub instances_tile_size: i64,
    pub instances_group_by: String,
    pub instances_group_by_asc: bool,
    pub instances_sort_by: String,
    pub instances_sort_by_asc: bool,
    pub show_news: bool,
    pub show_featured: bool,
    pub deletion_through_recycle_bin: bool,
    pub game_resolution: Option<String>,
    pub launcher_action_on_game_launch: String,
    pub show_app_close_warning: bool,
    pub java_custom_args: String,
    pub xmx: i64,
    pub xms: i64,
    pub default_instance_group: Option<i64>,
    pub pre_launch_hook: Option<String>,
    pub wrapper_command: Option<String>,
    pub post_exit_hook: Option<String>,
    pub is_first_launch: bool,
    pub auto_manage_java_system_profiles: bool,
    pub mod_platform_blacklist: String,
    pub mod_channels: String,
    pub terms_and_privacy_accepted: bool,
    pub terms_and_privacy_accepted_checksum: Option<String>,
    pub hashed_email_accepted: bool,
    pub gdl_account_uuid: Option<String>,
    pub gdl_account_status: Option<Vec<u8>>,
}

impl Appconfiguration {
    pub const TABLE: &'static str = "AppConfiguration";
    pub const ID: &'static str = "id";
    pub const THEME: &'static str = "theme";
    pub const REDUCEDMOTION: &'static str = "reducedMotion";
    pub const LANGUAGE: &'static str = "language";
    pub const DISCORDINTEGRATION: &'static str = "discordIntegration";
    pub const RELEASECHANNEL: &'static str = "releaseChannel";
    pub const LASTAPPVERSION: &'static str = "lastAppVersion";
    pub const ACTIVEACCOUNTUUID: &'static str = "activeAccountUuid";
    pub const CONCURRENTDOWNLOADS: &'static str = "concurrentDownloads";
    pub const DOWNLOADDEPENDENCIES: &'static str = "downloadDependencies";
    pub const INSTANCESTILESIZE: &'static str = "instancesTileSize";
    pub const INSTANCESGROUPBY: &'static str = "instancesGroupBy";
    pub const INSTANCESGROUPBYASC: &'static str = "instancesGroupByAsc";
    pub const INSTANCESSORTBY: &'static str = "instancesSortBy";
    pub const INSTANCESSORTBYASC: &'static str = "instancesSortByAsc";
    pub const SHOWNEWS: &'static str = "showNews";
    pub const SHOWFEATURED: &'static str = "showFeatured";
    pub const DELETIONTHROUGHRECYCLEBIN: &'static str = "deletionThroughRecycleBin";
    pub const GAMERESOLUTION: &'static str = "gameResolution";
    pub const LAUNCHERACTIONONGAMELAUNCH: &'static str = "launcherActionOnGameLaunch";
    pub const SHOWAPPCLOSEWARNING: &'static str = "showAppCloseWarning";
    pub const JAVACUSTOMARGS: &'static str = "javaCustomArgs";
    pub const XMX: &'static str = "xmx";
    pub const XMS: &'static str = "xms";
    pub const DEFAULTINSTANCEGROUP: &'static str = "defaultInstanceGroup";
    pub const PRELAUNCHHOOK: &'static str = "preLaunchHook";
    pub const WRAPPERCOMMAND: &'static str = "wrapperCommand";
    pub const POSTEXITHOOK: &'static str = "postExitHook";
    pub const ISFIRSTLAUNCH: &'static str = "isFirstLaunch";
    pub const AUTOMANAGEJAVASYSTEMPROFILES: &'static str = "autoManageJavaSystemProfiles";
    pub const MODPLATFORMBLACKLIST: &'static str = "modPlatformBlacklist";
    pub const MODCHANNELS: &'static str = "modChannels";
    pub const TERMSANDPRIVACYACCEPTED: &'static str = "termsAndPrivacyAccepted";
    pub const TERMSANDPRIVACYACCEPTEDCHECKSUM: &'static str = "termsAndPrivacyAcceptedChecksum";
    pub const HASHEDEMAILACCEPTED: &'static str = "hashedEmailAccepted";
    pub const GDLACCOUNTUUID: &'static str = "gdlAccountUuid";
    pub const GDLACCOUNTSTATUS: &'static str = "gdlAccountStatus";

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        theme: String,
        reduced_motion: bool,
        language: String,
        discord_integration: bool,
        release_channel: String,
        last_app_version: Option<String>,
        active_account_uuid: Option<String>,
        concurrent_downloads: i64,
        download_dependencies: bool,
        instances_tile_size: i64,
        instances_group_by: String,
        instances_group_by_asc: bool,
        instances_sort_by: String,
        instances_sort_by_asc: bool,
        show_news: bool,
        show_featured: bool,
        deletion_through_recycle_bin: bool,
        game_resolution: Option<String>,
        launcher_action_on_game_launch: String,
        show_app_close_warning: bool,
        java_custom_args: String,
        xmx: i64,
        xms: i64,
        default_instance_group: Option<i64>,
        pre_launch_hook: Option<String>,
        wrapper_command: Option<String>,
        post_exit_hook: Option<String>,
        is_first_launch: bool,
        auto_manage_java_system_profiles: bool,
        mod_platform_blacklist: String,
        mod_channels: String,
        terms_and_privacy_accepted: bool,
        terms_and_privacy_accepted_checksum: Option<String>,
        hashed_email_accepted: bool,
        gdl_account_uuid: Option<String>,
        gdl_account_status: Option<Vec<u8>>,
    ) -> Appconfiguration {
        Appconfiguration {
            id,
            theme,
            reduced_motion,
            language,
            discord_integration,
            release_channel,
            last_app_version,
            active_account_uuid,
            concurrent_downloads,
            download_dependencies,
            instances_tile_size,
            instances_group_by,
            instances_group_by_asc,
            instances_sort_by,
            instances_sort_by_asc,
            show_news,
            show_featured,
            deletion_through_recycle_bin,
            game_resolution,
            launcher_action_on_game_launch,
            show_app_close_warning,
            java_custom_args,
            xmx,
            xms,
            default_instance_group,
            pre_launch_hook,
            wrapper_command,
            post_exit_hook,
            is_first_launch,
            auto_manage_java_system_profiles,
            mod_platform_blacklist,
            mod_channels,
            terms_and_privacy_accepted,
            terms_and_privacy_accepted_checksum,
            hashed_email_accepted,
            gdl_account_uuid,
            gdl_account_status,
        }
    }

    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(Self::ID)?,
            theme: row.get(Self::THEME)?,
            reduced_motion: row.get(Self::REDUCEDMOTION)?,
            language: row.get(Self::LANGUAGE)?,
            discord_integration: row.get(Self::DISCORDINTEGRATION)?,
            release_channel: row.get(Self::RELEASECHANNEL)?,
            last_app_version: row.get(Self::LASTAPPVERSION)?,
            active_account_uuid: row.get(Self::ACTIVEACCOUNTUUID)?,
            concurrent_downloads: row.get(Self::CONCURRENTDOWNLOADS)?,
            download_dependencies: row.get(Self::DOWNLOADDEPENDENCIES)?,
            instances_tile_size: row.get(Self::INSTANCESTILESIZE)?,
            instances_group_by: row.get(Self::INSTANCESGROUPBY)?,
            instances_group_by_asc: row.get(Self::INSTANCESGROUPBYASC)?,
            instances_sort_by: row.get(Self::INSTANCESSORTBY)?,
            instances_sort_by_asc: row.get(Self::INSTANCESSORTBYASC)?,
            show_news: row.get(Self::SHOWNEWS)?,
            show_featured: row.get(Self::SHOWFEATURED)?,
            deletion_through_recycle_bin: row.get(Self::DELETIONTHROUGHRECYCLEBIN)?,
            game_resolution: row.get(Self::GAMERESOLUTION)?,
            launcher_action_on_game_launch: row.get(Self::LAUNCHERACTIONONGAMELAUNCH)?,
            show_app_close_warning: row.get(Self::SHOWAPPCLOSEWARNING)?,
            java_custom_args: row.get(Self::JAVACUSTOMARGS)?,
            xmx: row.get(Self::XMX)?,
            xms: row.get(Self::XMS)?,
            default_instance_group: row.get(Self::DEFAULTINSTANCEGROUP)?,
            pre_launch_hook: row.get(Self::PRELAUNCHHOOK)?,
            wrapper_command: row.get(Self::WRAPPERCOMMAND)?,
            post_exit_hook: row.get(Self::POSTEXITHOOK)?,
            is_first_launch: row.get(Self::ISFIRSTLAUNCH)?,
            auto_manage_java_system_profiles: row.get(Self::AUTOMANAGEJAVASYSTEMPROFILES)?,
            mod_platform_blacklist: row.get(Self::MODPLATFORMBLACKLIST)?,
            mod_channels: row.get(Self::MODCHANNELS)?,
            terms_and_privacy_accepted: row.get(Self::TERMSANDPRIVACYACCEPTED)?,
            terms_and_privacy_accepted_checksum: row.get(Self::TERMSANDPRIVACYACCEPTEDCHECKSUM)?,
            hashed_email_accepted: row.get(Self::HASHEDEMAILACCEPTED)?,
            gdl_account_uuid: row.get(Self::GDLACCOUNTUUID)?,
            gdl_account_status: row.get(Self::GDLACCOUNTSTATUS)?,
        })
    }

    pub fn columns() -> &'static [&'static str] {
        &[
            Self::ID,
            Self::THEME,
            Self::REDUCEDMOTION,
            Self::LANGUAGE,
            Self::DISCORDINTEGRATION,
            Self::RELEASECHANNEL,
            Self::LASTAPPVERSION,
            Self::ACTIVEACCOUNTUUID,
            Self::CONCURRENTDOWNLOADS,
            Self::DOWNLOADDEPENDENCIES,
            Self::INSTANCESTILESIZE,
            Self::INSTANCESGROUPBY,
            Self::INSTANCESGROUPBYASC,
            Self::INSTANCESSORTBY,
            Self::INSTANCESSORTBYASC,
            Self::SHOWNEWS,
            Self::SHOWFEATURED,
            Self::DELETIONTHROUGHRECYCLEBIN,
            Self::GAMERESOLUTION,
            Self::LAUNCHERACTIONONGAMELAUNCH,
            Self::SHOWAPPCLOSEWARNING,
            Self::JAVACUSTOMARGS,
            Self::XMX,
            Self::XMS,
            Self::DEFAULTINSTANCEGROUP,
            Self::PRELAUNCHHOOK,
            Self::WRAPPERCOMMAND,
            Self::POSTEXITHOOK,
            Self::ISFIRSTLAUNCH,
            Self::AUTOMANAGEJAVASYSTEMPROFILES,
            Self::MODPLATFORMBLACKLIST,
            Self::MODCHANNELS,
            Self::TERMSANDPRIVACYACCEPTED,
            Self::TERMSANDPRIVACYACCEPTEDCHECKSUM,
            Self::HASHEDEMAILACCEPTED,
            Self::GDLACCOUNTUUID,
            Self::GDLACCOUNTSTATUS,
        ]
    }

    pub fn get_by_id(conn: &rusqlite::Connection, id: i64) -> rusqlite::Result<Option<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!(
            "SELECT {} FROM {} WHERE {} = ?",
            columns,
            Self::TABLE,
            Self::ID
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {
        let columns = Self::columns().join(", ");
        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], Self::from_row)?;
        rows.collect()
    }
}

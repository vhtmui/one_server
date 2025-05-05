use chrono::{DateTime, NaiveTime, Utc};
use mysql_async::{Conn, Opts, Pool};
use mysql_async::{OptsBuilder, prelude::*};
use std::env;
use std::fmt::Debug;
use std::fs;
use std::io::Error;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct FileInfo {
    path: String,
    filename: String,
    created_at: DateTime<Utc>,
    modified_at: DateTime<Utc>,
    size: u64,
    parent_path: Option<String>,
}

impl FileInfo {
    /// 从PathBuf构造FileInfo
    fn from_path(path: &PathBuf) -> std::io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let created: DateTime<Utc> = metadata
            .created()
            .map(|t| t.into())
            .unwrap_or_else(|_| DateTime::UNIX_EPOCH);
        let modified: DateTime<Utc> = metadata
            .modified()
            .map(|t| t.into())
            .unwrap_or_else(|_| DateTime::UNIX_EPOCH);
        let size = metadata.len();
        let parent_path = path.parent().map(|p| p.display().to_string());

        Ok(FileInfo {
            path: path.display().to_string(),
            filename: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            created_at: created,
            modified_at: modified,
            size,
            parent_path,
        })
    }
}

mod db {
    use super::*;

    pub async fn init_pool() -> Pool {
        let url = env::var("DB_URL").expect("DB_URL must be set");
        Pool::new(url.as_str())
    }

    /// 批量插入文件信息，存在则更新time_last_written和file_size
    pub async fn insert_file_infos(conn: &mut Conn, infos: &[FileInfo]) -> mysql_async::Result<()> {
        if infos.is_empty() {
            return Ok(());
        }
        let mut sql = String::from(
            "INSERT INTO testdata.file_info (file_path, file_name, time_created, time_last_written, file_size, parent_directory) VALUES ",
        );
        let mut params = Vec::new();
        for (i, info) in infos.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            sql.push_str("(?, ?, ?, ?, ?, ?)");
            params.push(info.path.clone());
            params.push(info.filename.clone());
            params.push(info.created_at.format("%Y-%m-%d %H:%M:%S").to_string());
            params.push(info.modified_at.format("%Y-%m-%d %H:%M:%S").to_string());
            params.push(info.size.to_string());
            params.push(info.parent_path.clone().unwrap_or_else(|| "".to_string()));
        }
        sql.push_str(" ON DUPLICATE KEY UPDATE time_last_written=VALUES(time_last_written), file_size=VALUES(file_size)");
        conn.exec_drop(sql, params).await
    }
}

/// Example:
/// ```
/// process_paths(vec![PathBuf::from("/path/to/file1"), PathBuf::from("/path/to/file2")])
/// ```
pub async fn process_paths(paths: Vec<PathBuf>) -> Result<(), Error> {
    let pool = db::init_pool().await;
    let mut file_infos = Vec::new();
    let current_path = std::env::current_dir()?;

    for path in paths {
        if let Ok(info) = FileInfo::from_path(&path) {
            file_infos.push(info);
        } else {
            eprintln!();
            return Err(Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Failed to read file metadata for {:?}, current path is {}",
                    path,
                    current_path.display(),
                ),
            ));
        }
    }

    // 分批插入
    let batch_size = 1000;
    let mut idx = 0;
    while idx < file_infos.len() {
        let end = (idx + batch_size).min(file_infos.len());
        let batch = file_infos[idx..end].to_vec();
        let mut conn = match pool.get_conn().await {
            Ok(c) => c,
            Err(e) => {
                return Err(Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to get DB connection with {}", e),
                ));
            }
        };
        if let Err(e) = db::insert_file_infos(&mut conn, &batch).await {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to insert file info with {}", e),
            ));
        }
        idx = end;
    }
    Ok(())
}

#[test]
fn test_mysql_url() {
    let url = "mysql://q:1234.Com@10.50.3.70:3306/testdata";
    let _opts = Opts::from_url(url).unwrap();
}

#[test]
fn insert_file_info() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let base = std::env::temp_dir().join("test_asset");
        std::fs::create_dir_all(&base).unwrap();
        let mut paths = Vec::new();
        for i in 0..3 {
            let file = base.join(format!("file{}", i));
            std::fs::write(&file, b"test").unwrap();
            paths.push(file);
        }

        process_paths(paths).await.unwrap();

        std::fs::remove_dir_all(&base).unwrap();
    });
}

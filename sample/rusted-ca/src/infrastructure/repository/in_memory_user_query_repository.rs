//infrastructure/repository/in_memory_user_query_repository.rs
// SQLite Query Repository実装
// 2025/7/8

use crate::domain::entity::user::User;
use crate::domain::repository::user_query_repository::UserQueryRepositoryInterface;
use crate::domain::value_object::{
    birth_date::BirthDate, email::Email, pagination::*, password::Password, phone::Phone,
    user_id::UserId, user_name::UserName,
};
use crate::infrastructure::database::sqlite_connection::SqliteConnection;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{Row, params};

pub struct SqliteUserQueryRepository {
    db: SqliteConnection,
}

impl SqliteUserQueryRepository {
    pub fn new(db: SqliteConnection) -> Self {
        Self { db }
    }

    fn row_to_user(row: &Row) -> rusqlite::Result<User> {
        // データベースのカラム順序: id, email, name, password, phone, birth_date, created_at, last_login_at
        let id: String = row.get(0)?;
        let email: String = row.get(1)?;
        let name: String = row.get(2)?;
        let password: String = row.get(3)?;
        let phone: Option<String> = row.get(4)?;
        let birth_date: Option<String> = row.get(5)?;

        // Value Objectの構築
        let user_id = UserId::new(id);
        let email_vo =
            Email::new(email).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let name_vo = UserName::new(name)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let password_vo = Password::new(password)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        let phone_vo = match phone {
            Some(p) => Some(
                Phone::new(p).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
            ),
            None => None,
        };

        let birth_date_vo = match birth_date {
            Some(b) => Some(
                BirthDate::new(b)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
            ),
            None => None,
        };

        // Userエンティティの構築
        let user = User::new(
            user_id,
            email_vo,
            name_vo,
            password_vo,
            phone_vo,
            birth_date_vo,
        )
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        Ok(user)
    }

    fn count_with_filters(&self, _filters: &UserSearchFilters) -> rusqlite::Result<u64> {
        todo!("Implement count_with_filters")
    }
}

#[async_trait]
impl UserQueryRepositoryInterface for SqliteUserQueryRepository {
    async fn find_by_id(
        &self,
        id: &UserId,
    ) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>> {
        let id = id.clone();
        let result: Result<Option<User>, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let mut stmt = conn.prepare("SELECT * FROM users WHERE id = ?")?;
                let mut rows = stmt.query(params![id.0])?;
                if let Some(row) = rows.next()? {
                    Ok(Some(Self::row_to_user(row)?))
                } else {
                    Ok(None)
                }
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn find_by_email(
        &self,
        email: &Email,
    ) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>> {
        let email = email.clone();
        let result: Result<Option<User>, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let mut stmt = conn.prepare("SELECT * FROM users WHERE email = ?")?;
                let mut rows = stmt.query(params![email.0])?;
                if let Some(row) = rows.next()? {
                    Ok(Some(Self::row_to_user(row)?))
                } else {
                    Ok(None)
                }
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn exists_by_email(
        &self,
        email: &Email,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let email = email.clone();
        let result: Result<bool, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let mut stmt = conn.prepare("SELECT COUNT(*) FROM users WHERE email = ?")?;
                let count: i64 = stmt.query_row(params![email.0], |row| row.get(0))?;
                Ok(count > 0)
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn find_all(
        &self,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<User>, Box<dyn std::error::Error + Send + Sync>> {
        let pagination = pagination.clone();
        let offset = (pagination.page - 1) * pagination.limit;
        let result: Result<PaginatedResult<User>, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let total_count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
                let mut stmt =
                    conn.prepare("SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?")?;
                let mut rows = stmt.query(params![pagination.limit as i64, offset as i64])?;
                let mut users = Vec::new();
                while let Some(row) = rows.next()? {
                    users.push(Self::row_to_user(row)?);
                }
                let total_pages = ((total_count as u64 + pagination.limit as u64 - 1)
                    / pagination.limit as u64) as u32;
                Ok(PaginatedResult {
                    data: users,
                    pagination: PaginationInfo {
                        current_page: pagination.page,
                        total_pages,
                        total_count: total_count as u64,
                        per_page: pagination.limit,
                        has_next: pagination.page < total_pages,
                        has_prev: pagination.page > 1,
                    },
                })
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn count_total(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let result: Result<u64, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
                Ok(count as u64)
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn search_users(
        &self,
        filters: UserSearchFilters,
        sort: SortParams,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<User>, Box<dyn std::error::Error + Send + Sync>> {
        let filters = filters.clone();
        let sort = sort.clone();
        let pagination = pagination.clone();
        let offset = (pagination.page - 1) * pagination.limit;
        let result: Result<PaginatedResult<User>, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let mut sql = "SELECT * FROM users WHERE 1=1".to_string();
                let mut params_vec = Vec::new();
                if let Some(email_domain) = &filters.email_domain {
                    sql.push_str(" AND email LIKE ?");
                    params_vec.push(format!("%{}", email_domain));
                }
                if let Some(name_contains) = &filters.name_contains {
                    sql.push_str(" AND name LIKE ?");
                    params_vec.push(format!("%{}%", name_contains));
                }
                if let Some(created_after) = &filters.created_after {
                    sql.push_str(" AND created_at >= ?");
                    params_vec.push(created_after.clone());
                }
                if let Some(created_before) = &filters.created_before {
                    sql.push_str(" AND created_at <= ?");
                    params_vec.push(created_before.clone());
                }
                if let Some(has_phone) = &filters.has_phone {
                    if *has_phone {
                        sql.push_str(" AND phone IS NOT NULL");
                    } else {
                        sql.push_str(" AND phone IS NULL");
                    }
                }
                let order_str = match sort.order {
                    SortOrder::Asc => "ASC",
                    SortOrder::Desc => "DESC",
                };
                sql.push_str(&format!(" ORDER BY {} {}", sort.field, order_str));
                sql.push_str(&format!(" LIMIT {} OFFSET {}", pagination.limit, offset));
                let mut stmt = conn.prepare(&sql)?;
                let mut rows = stmt.query(rusqlite::params_from_iter(params_vec.iter()))?;
                let mut users = Vec::new();
                while let Some(row) = rows.next()? {
                    users.push(Self::row_to_user(row)?);
                }
                let count_sql_binding = sql.replace("SELECT *", "SELECT COUNT(*)");
                let count_sql = count_sql_binding.split("ORDER BY").next().unwrap();
                let mut count_stmt = conn.prepare(count_sql)?;
                let total_count: i64 = count_stmt
                    .query_row(rusqlite::params_from_iter(params_vec.iter()), |row| {
                        row.get(0)
                    })?;
                let total_pages = ((total_count as u64 + pagination.limit as u64 - 1)
                    / pagination.limit as u64) as u32;
                Ok(PaginatedResult {
                    data: users,
                    pagination: PaginationInfo {
                        current_page: pagination.page,
                        total_pages,
                        total_count: total_count as u64,
                        per_page: pagination.limit,
                        has_next: pagination.page < total_pages,
                        has_prev: pagination.page > 1,
                    },
                })
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn count_registrations_in_period(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let start = start.clone();
        let end = end.clone();
        let result: Result<u64, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM users WHERE created_at BETWEEN ? AND ?",
                    params![start.to_rfc3339(), end.to_rfc3339()],
                    |row| row.get(0),
                )?;
                Ok(count as u64)
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn count_active_users_in_period(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let start = start.clone();
        let end = end.clone();
        let result: Result<u64, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(DISTINCT id) FROM users WHERE last_login_at BETWEEN ? AND ?",
                    params![start.to_rfc3339(), end.to_rfc3339()],
                    |row| row.get(0),
                )?;
                Ok(count as u64)
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn get_registration_trend(
        &self,
        period: TimePeriod,
        granularity: TimeGranularity,
    ) -> Result<Vec<TimeSeriesPoint>, Box<dyn std::error::Error + Send + Sync>> {
        let period = period.clone();
        let granularity = granularity.clone();
        let result: Result<Vec<TimeSeriesPoint>, rusqlite::Error> = self
            .db
            .execute_query(move |conn| {
                let (date_format, _interval) = match granularity {
                    TimeGranularity::Hour => ("%Y-%m-%d %H:00:00", "1 hour"),
                    TimeGranularity::Day => ("%Y-%m-%d", "1 day"),
                    TimeGranularity::Week => ("%Y-%W", "1 week"),
                    TimeGranularity::Month => ("%Y-%m", "1 month"),
                    TimeGranularity::Minute => ("%Y-%m-%d %H:%M:00", "1 minute"),
                };
                let period_hours = match period {
                    TimePeriod::Hour => 1,
                    TimePeriod::Day => 24,
                    TimePeriod::Week => 24 * 7,
                    TimePeriod::Month => 24 * 30,
                    TimePeriod::Year => 24 * 365,
                };
                let sql = format!(
                    "SELECT strftime('{}', created_at) as time_bucket, COUNT(*) as count \
                 FROM users \
                 WHERE created_at >= datetime('now', '-{} hours')\
                 GROUP BY time_bucket \
                 ORDER BY time_bucket",
                    date_format, period_hours
                );
                let mut stmt = conn.prepare(&sql)?;
                let mut rows = stmt.query([])?;
                let mut points = Vec::new();
                while let Some(row) = rows.next()? {
                    points.push(TimeSeriesPoint {
                        timestamp: row.get::<_, String>(0)?,
                        value: row.get::<_, i64>(1)? as u64,
                        sample_count: None,
                    });
                }
                Ok(points)
            })
            .await;
        result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

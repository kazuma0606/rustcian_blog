//domain/repository/user_query_repository.rs
// Query Repository トレイト
// 2025/7/8

use crate::domain::entity::user::User;
use crate::domain::value_object::{email::Email, pagination::*, user_id::UserId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[async_trait]
pub trait UserQueryRepositoryInterface: Send + Sync {
    // 基本検索
    async fn find_by_id(
        &self,
        id: &UserId,
    ) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn find_by_email(
        &self,
        email: &Email,
    ) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn exists_by_email(
        &self,
        email: &Email,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    // 一覧・ページング
    async fn find_all(
        &self,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<User>, Box<dyn std::error::Error + Send + Sync>>;
    async fn count_total(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    // 高度な検索（API 16に対応）
    async fn search_users(
        &self,
        filters: UserSearchFilters,
        sort: SortParams,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<User>, Box<dyn std::error::Error + Send + Sync>>;

    // Analytics用（API 17-18に対応）
    async fn count_registrations_in_period(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
    async fn count_active_users_in_period(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_registration_trend(
        &self,
        period: TimePeriod,
        granularity: TimeGranularity,
    ) -> Result<Vec<TimeSeriesPoint>, Box<dyn std::error::Error + Send + Sync>>;
}

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// Custom deserializer for date strings to OffsetDateTime
mod date_format {
    use serde::{self, Deserialize, Deserializer};
    use time::{Date, OffsetDateTime, Time, UtcOffset};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        match s {
            Some(s) => {
                // Try to parse as date-only string (YYYY-MM-DD)
                if let Ok(date) =
                    Date::parse(&s, &time::format_description::well_known::Iso8601::DEFAULT)
                {
                    let datetime = date.with_time(Time::MIDNIGHT).assume_offset(UtcOffset::UTC);
                    Ok(Some(datetime))
                } else {
                    // Try to parse as full datetime
                    OffsetDateTime::parse(
                        &s,
                        &time::format_description::well_known::Iso8601::DEFAULT,
                    )
                    .map(Some)
                    .map_err(serde::de::Error::custom)
                }
            }
            None => Ok(None),
        }
    }
}

// Custom serializer for OffsetDateTime to ISO 8601 string
mod iso8601_option {
    use serde::{self, Serializer};
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    pub fn serialize<S>(date: &Option<OffsetDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(dt) => {
                let s = dt.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
                serializer.serialize_some(&s)
            }
            None => serializer.serialize_none(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub full_name: String,
    pub phone_num: Option<String>,
    pub image: Option<String>,
    pub points: i32,
    pub rank: i32,
    pub role: String,
    pub jupyterhub_username: Option<String>,
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    #[serde(rename = "fullName")]
    pub full_name: String,
    #[serde(rename = "phoneNum")]
    pub phone_num: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub email: String,
    pub image: Option<String>,
    pub role: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Leaderboard {
    pub id: i32,
    pub title: String,
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LeaderboardEntry {
    pub name: String,
    pub points: i32,
}

#[derive(Debug, Serialize)]
pub struct LeaderboardResponse {
    pub id: i32,
    pub title: String,
    pub entries: Vec<LeaderboardEntry>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Resource {
    pub id: i32,
    pub title: String,
    pub provider: String,
    pub cover_image: Option<String>,
    pub instructor_name: String,
    pub instructor_image: Option<String>,
    pub notion_url: Option<String>,
    pub visible: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Quote {
    pub id: i32,
    pub text: String,
    pub author: String,
    pub visible: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Certificate {
    pub id: i32,
    pub level: String,
    pub title: String,
    pub course_title: String,
    pub cover_image: Option<String>,
    pub first_name: String,
    pub second_name: String,
    pub coursera_url: Option<String>,
    pub youtube_url: Option<String>,
    pub visible: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct ResourceListResponse {
    pub id: i32,
    pub title: String,
    pub provider: String,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    pub instructor: InstructorResponse,
}

#[derive(Debug, Serialize)]
pub struct ResourceDetailResponse {
    pub id: i32,
    pub title: String,
    pub provider: String,
    #[serde(rename = "notionUrl")]
    pub notion_url: Option<String>,
    pub instructor: InstructorResponse,
    pub quote: Option<QuoteResponse>,
}

#[derive(Debug, Serialize)]
pub struct CertificateListResponse {
    pub id: i32,
    pub level: String,
    pub title: String,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "secondName")]
    pub second_name: String,
}

#[derive(Debug, Serialize)]
pub struct CertificateDetailResponse {
    pub id: i32,
    pub level: String,
    pub title: String,
    #[serde(rename = "courseTitle")]
    pub course_title: String,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "secondName")]
    pub second_name: String,
    #[serde(rename = "courseraUrl")]
    pub coursera_url: Option<String>,
    #[serde(rename = "youtubeUrl")]
    pub youtube_url: Option<String>,
    pub quote: Option<QuoteResponse>,
}

#[derive(Debug, Serialize)]
pub struct InstructorResponse {
    pub name: String,
    pub image: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    pub text: String,
    pub author: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct Challenge {
    pub id: i32,
    pub week: i32,
    pub title: String,
    pub description: String,
    pub challenge_url: String,
    pub is_current: bool,
    pub start_date: Option<time::OffsetDateTime>,
    pub end_date: Option<time::OffsetDateTime>,
    pub visible: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub id: i32,
    pub week: i32,
    pub title: String,
    pub description: String,
    #[serde(rename = "challengeUrl")]
    pub challenge_url: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ChallengeLeaderboardEntry {
    pub id: Uuid,
    pub name: String,
    pub points: i32,
    pub image: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserStats {
    pub id: Uuid,
    pub user_id: Uuid,
    pub best_subject: Option<String>,
    pub improveable: Option<String>,
    pub quickest_hunter: i32,
    pub challenges_taken: i32,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub rank: i32,
    pub name: String,
    pub points: i32,
    pub image: Option<String>,
    pub stats: UserStatsResponse,
}

#[derive(Debug, Serialize)]
pub struct UserStatsResponse {
    #[serde(rename = "bestSubject")]
    pub best_subject: Option<String>,
    pub improveable: Option<String>,
    #[serde(rename = "quickestHunter")]
    pub quickest_hunter: i32,
    #[serde(rename = "challengesTaken")]
    pub challenges_taken: i32,
}

#[derive(Debug, Deserialize)]
pub struct ContactRequest {
    pub name: String,
    pub email: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ContactResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AdminResourceResponse {
    pub id: i32,
    pub title: String,
    pub provider: String,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    #[serde(rename = "notionUrl")]
    pub notion_url: Option<String>,
    pub instructor: Option<AdminInstructorResponse>,
    pub quote: Option<AdminQuoteResponse>,
    pub visible: bool,
    #[serde(rename = "createdAt")]
    pub created_at: time::OffsetDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct AdminCertificateResponse {
    pub id: i32,
    pub level: String,
    pub title: String,
    #[serde(rename = "courseTitle")]
    pub course_title: String,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "secondName")]
    pub second_name: String,
    #[serde(rename = "courseraUrl")]
    pub coursera_url: Option<String>,
    #[serde(rename = "youtubeUrl")]
    pub youtube_url: Option<String>,
    pub visible: bool,
    #[serde(rename = "createdAt")]
    pub created_at: time::OffsetDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct AdminInstructorResponse {
    pub name: String,
    pub image: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminQuoteResponse {
    pub text: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminCreateResourceRequest {
    pub title: String,
    pub provider: String,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    #[serde(rename = "notionUrl")]
    pub notion_url: Option<String>,
    pub instructor: Option<AdminInstructorRequest>,
    pub quote: Option<AdminQuoteRequest>,
    pub visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminCreateCertificateRequest {
    pub level: String,
    pub title: String,
    #[serde(rename = "courseTitle")]
    pub course_title: String,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: String,
    #[serde(rename = "secondName")]
    pub second_name: String,
    #[serde(rename = "courseraUrl")]
    pub coursera_url: Option<String>,
    #[serde(rename = "youtubeUrl")]
    pub youtube_url: Option<String>,
    pub visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminUpdateResourceRequest {
    pub title: Option<String>,
    pub provider: Option<String>,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    #[serde(rename = "notionUrl")]
    pub notion_url: Option<String>,
    pub instructor: Option<AdminInstructorRequest>,
    pub quote: Option<AdminQuoteRequest>,
    pub visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminUpdateCertificateRequest {
    pub level: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "courseTitle")]
    pub course_title: Option<String>,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<String>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "secondName")]
    pub second_name: Option<String>,
    #[serde(rename = "courseraUrl")]
    pub coursera_url: Option<String>,
    #[serde(rename = "youtubeUrl")]
    pub youtube_url: Option<String>,
    pub visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminInstructorRequest {
    pub name: String,
    pub image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminQuoteRequest {
    pub text: String,
    pub author: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminVisibilityRequest {
    pub visible: bool,
}

#[derive(Debug, Serialize)]
pub struct AdminChallengeResponse {
    pub id: i32,
    pub title: String,
    pub description: String,
    #[serde(rename = "startDate")]
    pub start_date: Option<time::OffsetDateTime>,
    #[serde(rename = "endDate")]
    pub end_date: Option<time::OffsetDateTime>,
    pub visible: bool,
    #[serde(rename = "createdAt")]
    pub created_at: time::OffsetDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct AdminCreateChallengeRequest {
    pub title: String,
    pub description: String,
    pub week: Option<i32>,
    #[serde(rename = "challengeUrl")]
    pub challenge_url: Option<String>,
    #[serde(rename = "startDate", deserialize_with = "date_format::deserialize")]
    pub start_date: Option<time::OffsetDateTime>,
    #[serde(rename = "endDate", deserialize_with = "date_format::deserialize")]
    pub end_date: Option<time::OffsetDateTime>,
    pub visible: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminUpdateChallengeRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub week: Option<i32>,
    #[serde(rename = "challengeUrl")]
    pub challenge_url: Option<String>,
    #[serde(rename = "startDate", deserialize_with = "date_format::deserialize")]
    pub start_date: Option<time::OffsetDateTime>,
    #[serde(rename = "endDate", deserialize_with = "date_format::deserialize")]
    pub end_date: Option<time::OffsetDateTime>,
    pub visible: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct AdminItemResponse<T> {
    pub item: T,
}

#[derive(Debug, Serialize)]
pub struct AdminItemsResponse<T> {
    pub items: Vec<T>,
}

#[derive(Debug, Serialize)]
pub struct AdminSuccessResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(rename = "fullName")]
    pub full_name: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateProfileResponse {
    pub id: Uuid,
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub email: String,
    pub image: Option<String>,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct UploadAvatarResponse {
    #[serde(rename = "imageUrl")]
    pub image_url: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    #[serde(rename = "currentPassword")]
    pub current_password: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct UpdatePasswordResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct CompleteProfileRequest {
    pub university: String,
    pub major: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteProfileResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct SetPasswordRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SetPasswordResponse {
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

// ============================================
// JupyterHub / nbgrader Challenge Integration
// ============================================

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ChallengeNotebook {
    pub id: i32,
    pub challenge_id: i32,
    pub assignment_name: String,
    pub notebook_filename: String,
    pub notebook_path: String,
    pub max_points: i32,
    pub cpu_limit: f64,
    pub memory_limit: String,
    pub time_limit_minutes: i32,
    pub network_disabled: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ChallengeSubmission {
    pub id: Uuid,
    pub user_id: Uuid,
    pub challenge_id: i32,
    pub notebook_id: i32,
    pub status: String,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub points_awarded: i32,
    pub points_credited: bool,
    pub nbgrader_submission_id: Option<String>,
    pub started_at: Option<time::OffsetDateTime>,
    pub submitted_at: Option<time::OffsetDateTime>,
    pub graded_at: Option<time::OffsetDateTime>,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    NotStarted,
    InProgress,
    Submitted,
    Grading,
    Graded,
    Error,
}

impl SubmissionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubmissionStatus::NotStarted => "not_started",
            SubmissionStatus::InProgress => "in_progress",
            SubmissionStatus::Submitted => "submitted",
            SubmissionStatus::Grading => "grading",
            SubmissionStatus::Graded => "graded",
            SubmissionStatus::Error => "error",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "not_started" => Some(SubmissionStatus::NotStarted),
            "in_progress" => Some(SubmissionStatus::InProgress),
            "submitted" => Some(SubmissionStatus::Submitted),
            "grading" => Some(SubmissionStatus::Grading),
            "graded" => Some(SubmissionStatus::Graded),
            "error" => Some(SubmissionStatus::Error),
            _ => None,
        }
    }
}

// API Request/Response types for Challenge Notebooks

#[derive(Debug, Serialize)]
pub struct ChallengeNotebookResponse {
    pub id: i32,
    #[serde(rename = "challengeId")]
    pub challenge_id: i32,
    #[serde(rename = "assignmentName")]
    pub assignment_name: String,
    #[serde(rename = "notebookFilename")]
    pub notebook_filename: String,
    #[serde(rename = "maxPoints")]
    pub max_points: i32,
    #[serde(rename = "timeLimitMinutes")]
    pub time_limit_minutes: i32,
}

#[derive(Debug, Serialize)]
pub struct ChallengeWithNotebookResponse {
    pub id: i32,
    pub week: i32,
    pub title: String,
    pub description: String,
    #[serde(rename = "hasNotebook")]
    pub has_notebook: bool,
    #[serde(rename = "maxPoints")]
    pub max_points: Option<i32>,
    #[serde(rename = "timeLimitMinutes")]
    pub time_limit_minutes: Option<i32>,
    #[serde(rename = "startDate", serialize_with = "iso8601_option::serialize")]
    pub start_date: Option<time::OffsetDateTime>,
    #[serde(rename = "endDate", serialize_with = "iso8601_option::serialize")]
    pub end_date: Option<time::OffsetDateTime>,
}

#[derive(Debug, Serialize)]
pub struct UserSubmissionResponse {
    pub id: Uuid,
    #[serde(rename = "challengeId")]
    pub challenge_id: i32,
    pub status: String,
    pub score: Option<f64>,
    #[serde(rename = "maxScore")]
    pub max_score: Option<f64>,
    #[serde(rename = "pointsAwarded")]
    pub points_awarded: i32,
    #[serde(rename = "startedAt", serialize_with = "iso8601_option::serialize")]
    pub started_at: Option<time::OffsetDateTime>,
    #[serde(rename = "submittedAt", serialize_with = "iso8601_option::serialize")]
    pub submitted_at: Option<time::OffsetDateTime>,
    #[serde(rename = "gradedAt", serialize_with = "iso8601_option::serialize")]
    pub graded_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Serialize)]
pub struct StartChallengeResponse {
    pub success: bool,
    #[serde(rename = "jupyterhubUrl")]
    pub jupyterhub_url: String,
    #[serde(rename = "submissionId")]
    pub submission_id: Uuid,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct NbgraderWebhookPayload {
    #[serde(rename = "assignmentName")]
    pub assignment_name: String,
    #[serde(rename = "studentId")]
    pub student_id: String,
    #[serde(rename = "submissionId")]
    pub submission_id: Option<String>,
    pub score: f64,
    #[serde(rename = "maxScore")]
    pub max_score: f64,
    pub timestamp: Option<String>,
    /// Secret key to verify the webhook is from JupyterHub
    #[serde(rename = "webhookSecret")]
    pub webhook_secret: String,
}

#[derive(Debug, Serialize)]
pub struct NbgraderWebhookResponse {
    pub success: bool,
    #[serde(rename = "pointsAwarded")]
    pub points_awarded: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitChallengeResponse {
    pub success: bool,
    pub message: String,
    pub status: String,
}

// Admin types for notebook management

#[derive(Debug, Serialize)]
pub struct AdminChallengeNotebookResponse {
    pub id: i32,
    #[serde(rename = "challengeId")]
    pub challenge_id: i32,
    #[serde(rename = "assignmentName")]
    pub assignment_name: String,
    #[serde(rename = "notebookFilename")]
    pub notebook_filename: String,
    #[serde(rename = "notebookPath")]
    pub notebook_path: String,
    #[serde(rename = "maxPoints")]
    pub max_points: i32,
    #[serde(rename = "cpuLimit")]
    pub cpu_limit: f64,
    #[serde(rename = "memoryLimit")]
    pub memory_limit: String,
    #[serde(rename = "timeLimitMinutes")]
    pub time_limit_minutes: i32,
    #[serde(rename = "networkDisabled")]
    pub network_disabled: bool,
    #[serde(rename = "createdAt")]
    pub created_at: time::OffsetDateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: time::OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct AdminCreateNotebookRequest {
    #[serde(rename = "challengeId")]
    pub challenge_id: i32,
    #[serde(rename = "assignmentName")]
    pub assignment_name: String,
    #[serde(rename = "maxPoints")]
    pub max_points: Option<i32>,
    #[serde(rename = "cpuLimit")]
    pub cpu_limit: Option<f64>,
    #[serde(rename = "memoryLimit")]
    pub memory_limit: Option<String>,
    #[serde(rename = "timeLimitMinutes")]
    pub time_limit_minutes: Option<i32>,
    #[serde(rename = "networkDisabled")]
    pub network_disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AdminUpdateNotebookRequest {
    #[serde(rename = "assignmentName")]
    pub assignment_name: Option<String>,
    #[serde(rename = "maxPoints")]
    pub max_points: Option<i32>,
    #[serde(rename = "cpuLimit")]
    pub cpu_limit: Option<f64>,
    #[serde(rename = "memoryLimit")]
    pub memory_limit: Option<String>,
    #[serde(rename = "timeLimitMinutes")]
    pub time_limit_minutes: Option<i32>,
    #[serde(rename = "networkDisabled")]
    pub network_disabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct AdminSubmissionResponse {
    pub id: Uuid,
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userEmail")]
    pub user_email: String,
    #[serde(rename = "challengeId")]
    pub challenge_id: i32,
    #[serde(rename = "challengeTitle")]
    pub challenge_title: String,
    pub status: String,
    pub score: Option<f64>,
    #[serde(rename = "maxScore")]
    pub max_score: Option<f64>,
    #[serde(rename = "pointsAwarded")]
    pub points_awarded: i32,
    #[serde(rename = "pointsCredited")]
    pub points_credited: bool,
    #[serde(rename = "startedAt", serialize_with = "iso8601_option::serialize")]
    pub started_at: Option<time::OffsetDateTime>,
    #[serde(rename = "submittedAt", serialize_with = "iso8601_option::serialize")]
    pub submitted_at: Option<time::OffsetDateTime>,
    #[serde(rename = "gradedAt", serialize_with = "iso8601_option::serialize")]
    pub graded_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ChallengeSubmissionLeaderboardEntry {
    pub challenge_id: i32,
    pub user_id: Uuid,
    pub full_name: String,
    pub image: Option<String>,
    pub points_awarded: i32,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub status: String,
    pub graded_at: Option<time::OffsetDateTime>,
    pub challenge_rank: i64,
}

// Admin JupyterHub access response
#[derive(Debug, Serialize)]
pub struct AdminJupyterHubAccessResponse {
    pub success: bool,
    #[serde(rename = "jupyterhubUrl")]
    pub jupyterhub_url: String,
    pub token: String,
    pub message: String,
}

// Response for syncing notebook to nbgrader
#[derive(Debug, Serialize)]
pub struct AdminSyncNotebookResponse {
    pub success: bool,
    pub message: String,
}

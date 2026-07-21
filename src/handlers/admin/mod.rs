pub mod articles;
pub mod challenges;
pub mod contact;
pub mod notebooks;
pub mod submissions;
pub mod upload;

pub use articles::{
    admin_create_article_multipart, admin_delete_article, admin_get_article_by_id,
    admin_get_articles, admin_patch_article_visibility, admin_update_article_multipart,
};
pub use challenges::{
    admin_create_challenge, admin_delete_challenge, admin_get_challenge_by_id,
    admin_get_challenges, admin_patch_challenge_visibility, admin_update_challenge,
};
pub use contact::admin_get_contact_messages;
pub use notebooks::{
    admin_create_notebook_multipart, admin_delete_notebook, admin_get_notebook_by_challenge,
    admin_get_notebooks, admin_update_notebook,
};
pub use submissions::{
    admin_delete_submission, admin_get_submission_access, admin_get_submission_file,
    admin_get_submissions, admin_grade_submission, admin_grant_attempts,
};

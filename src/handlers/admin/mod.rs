#[path = "certificates/mod.rs"]
pub mod certificates;
#[path = "challenges/mod.rs"]
pub mod challenges;
#[path = "notebooks/mod.rs"]
pub mod notebooks;
#[path = "resources/mod.rs"]
pub mod resources;
#[path = "submissions/mod.rs"]
pub mod submissions;

pub use certificates::{
    admin_create_certificate, admin_create_certificate_multipart, admin_delete_certificate,
    admin_get_certificate_by_id, admin_get_certificates, admin_patch_certificate_visibility,
    admin_update_certificate, admin_update_certificate_multipart,
};
pub use challenges::{
    admin_create_challenge, admin_delete_challenge, admin_get_challenge_by_id,
    admin_get_challenges, admin_patch_challenge_visibility, admin_update_challenge,
};
pub use notebooks::{
    admin_create_notebook_multipart, admin_delete_notebook, admin_get_notebook_by_challenge,
    admin_get_notebook_edit_url, admin_get_notebooks, admin_sync_notebook_to_nbgrader,
    admin_update_notebook,
};
pub use resources::{
    admin_create_resource, admin_create_resource_multipart, admin_delete_resource,
    admin_get_resource_by_id, admin_get_resources, admin_patch_resource_visibility,
    admin_update_resource, admin_update_resource_multipart,
};
pub use submissions::{
    admin_get_submission_access, admin_get_submission_file, admin_get_submissions,
    admin_grade_submission,
};

mod save_uploaded_file;

pub mod admin_get_resources;
pub mod admin_get_resource_by_id;
pub mod admin_create_resource;
pub mod admin_update_resource;
pub mod admin_delete_resource;
pub mod admin_patch_resource_visibility;
pub mod admin_create_resource_multipart;
pub mod admin_update_resource_multipart;

pub use admin_get_resources::admin_get_resources;
pub use admin_get_resource_by_id::admin_get_resource_by_id;
pub use admin_create_resource::admin_create_resource;
pub use admin_update_resource::admin_update_resource;
pub use admin_delete_resource::admin_delete_resource;
pub use admin_patch_resource_visibility::admin_patch_resource_visibility;
pub use admin_create_resource_multipart::admin_create_resource_multipart;
pub use admin_update_resource_multipart::admin_update_resource_multipart;

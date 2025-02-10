use crate::resource::Resource;

pub struct Device {
    pub id: String,
    pub content_resource: Resource,
}

pub struct Storage;

impl Storage {
    pub fn device_by_id(&self, id: &str) -> Option<Device> {
        if id == "test" {
            return Some(Device {
                id: "test".into(),
                content_resource: Resource::self_hosted_content("test"),
            });
        } else if id == "ticktick" {
            return Some(Device {
                id: "ticktick".into(),
                content_resource: Resource::self_hosted_content("ticktick"),
            });
        }
        None
    }
}

use crate::vulkan::VkState;


// Since this crate is all about rendering, it's hopefully clear
// this is about the renderer state. A shorter name is nice, because this is going to be
// passed around a *lot*.
//
// This has everything app-specific, unlike VkState (which has the obligatory Vulkan state) or
// the Renderer struct which glues everything together and has details (such as sync) that isn't
// relevant to pass around.
pub struct State {

}

pub fn init(vk: &VkState) -> anyhow::Result<State> {

    Ok(State {

    })
}
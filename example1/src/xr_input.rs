use gl_thin::errors::{Wrappable, XrErrorWrapped};
use gl_thin::openxr_helpers::Backend;
use openxr::{
    Action, ActionSet, ActiveActionSet, Binding, Instance, Session, Space, SpaceLocation,
};
use openxr_sys::{Path, Posef, Time};

pub struct XrInputs {
    pub action_set: ActionSet,
    pub user_hand_right: Path,
    pub controller_1: Action<Posef>,
    pub controller_space_1: Space,
}

impl XrInputs {
    pub fn new(instance: &Instance, xr_session: &Session<Backend>) -> Result<Self, XrErrorWrapped> {
        let action_set = instance
            .create_action_set("pants", "pants", 0)
            .annotate_if_err(Some(instance), "failed to create_action_set")?;

        //

        let user_hand_left = instance
            .string_to_path("/user/hand/left")
            .annotate_if_err(Some(instance), "failed to ")?;
        let user_hand_right = instance
            .string_to_path("/user/hand/right")
            .annotate_if_err(Some(instance), "failed to ")?;
        let pose_action = action_set
            .create_action::<Posef>(
                "hand_pose",
                "controller 1",
                &[user_hand_left, user_hand_right],
            )
            .annotate_if_err(Some(instance), "failed to ")?;
        let left_grip_pose = instance
            .string_to_path("/user/hand/left/input/grip/pose")
            .annotate_if_err(Some(instance), "failed to ")?;
        let right_grip_pose = instance
            .string_to_path("/user/hand/right/input/grip/pose")
            .annotate_if_err(Some(instance), "failed to ")?;
        let bindings = [
            Binding::new(&pose_action, left_grip_pose),
            Binding::new(&pose_action, right_grip_pose),
        ];
        {
            let interaction_profile = instance
                .string_to_path("/interaction_profiles/khr/simple_controller")
                .annotate_if_err(Some(instance), "failed to ")?;

            instance
                .suggest_interaction_profile_bindings(interaction_profile, &bindings)
                .annotate_if_err(Some(instance), "failed to ")?;
        }

        {
            let interaction_profile = instance
                .string_to_path("/interaction_profiles/oculus/touch_controller")
                .annotate_if_err(Some(instance), "failed to ")?;
            instance
                .suggest_interaction_profile_bindings(interaction_profile, &bindings)
                .annotate_if_err(Some(instance), "failed to ")?;
        }

        let mut posef = Posef::default();
        posef.orientation.w = 1.0;
        let controller_space_1 = pose_action
            .create_space(xr_session.clone(), user_hand_right, posef)
            .annotate_if_err(Some(instance), "failed to ")?;

        //

        xr_session
            .attach_action_sets(&[&action_set])
            .annotate_if_err(Some(instance), "failed to attach_action_sets")?;

        Ok(Self {
            action_set,
            user_hand_right,
            controller_1: pose_action,
            controller_space_1,
        })
    }

    pub fn sync_actions(&self, xr_session: &Session<Backend>) -> openxr::Result<()> {
        xr_session.sync_actions(&[ActiveActionSet::new(&self.action_set)])
    }

    pub fn controller_1_locate(
        &self,
        base: &Space,
        predicted_display_time: Time,
    ) -> openxr::Result<SpaceLocation> {
        self.controller_space_1.locate(base, predicted_display_time)
    }

    pub fn controller_1_locate_if_active<G>(
        &self,
        xr_session: &Session<G>,
        base: &Space,
        predicted_display_time: Time,
    ) -> Option<SpaceLocation> {
        if self
            .controller_1
            .is_active(xr_session, self.user_hand_right)
            .unwrap()
        {
            self.controller_1_locate(base, predicted_display_time).ok()
        } else {
            None
        }
    }
}

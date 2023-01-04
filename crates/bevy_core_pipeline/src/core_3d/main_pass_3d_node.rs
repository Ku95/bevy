use crate::{
    clear_color::{ClearColor, ClearColorConfig},
    core_3d::{AlphaMask3d, Camera3d, Opaque3d, Transparent3d},
};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{RenderPhase, TrackedRenderPass},
    render_resource::{LoadOp, Operations},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};

pub struct MainPass3dNode {
    query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<Opaque3d>,
            &'static RenderPhase<AlphaMask3d>,
            &'static RenderPhase<Transparent3d>,
            &'static Camera3d,
            &'static ViewTarget,
            &'static ViewDepthTexture,
        ),
        With<ExtractedView>,
    >,
}

impl MainPass3dNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl Node for MainPass3dNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(MainPass3dNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (camera, opaque_phase, alpha_mask_phase, transparent_phase, camera_3d, target, depth) =
            match self.query.get_manual(world, view_entity) {
                Ok(query) => query,
                Err(_) => {
                    return Ok(());
                } // No window
            };

        // Always run opaque pass to ensure screen is cleared
        {
            // Run the opaque pass, sorted front-to-back

            // NOTE: The opaque pass loads the color buffer as well as writing to it.
            let color_ops = Operations {
                load: match camera_3d.clear_color {
                    ClearColorConfig::Default => {
                        LoadOp::Clear(world.resource::<ClearColor>().0.into())
                    }
                    ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                    ClearColorConfig::None => LoadOp::Load,
                },
                store: true,
            };

            let depth_ops = Some(Operations {
                // NOTE: 0.0 is the far plane due to bevy's use of reverse-z projections.
                load: camera_3d.depth_load_op.clone().into(),
                store: true,
            });

            let mut render_pass = TrackedRenderPass::create_for_camera(
                render_context,
                "main_opaque_pass_3d",
                view_entity,
                target,
                color_ops,
                Some(depth),
                depth_ops,
                &camera.viewport,
            );

            render_pass.render_phase(opaque_phase, world);
        }

        if !alpha_mask_phase.items.is_empty() {
            // Run the alpha mask pass, sorted front-to-back

            let mut render_pass = TrackedRenderPass::create_for_camera(
                render_context,
                "main_alpha_mask_pass_3d",
                view_entity,
                target,
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
                Some(depth),
                Some(Operations {
                    load: LoadOp::Load,
                    store: true,
                }),
                &camera.viewport,
            );

            render_pass.render_phase(alpha_mask_phase, world);
        }

        if !transparent_phase.items.is_empty() {
            // Run the transparent pass, sorted back-to-front

            // NOTE: For the transparent pass we load the depth buffer. There should be no
            // need to write to it, but store is set to `true` as a workaround for issue #3776,
            // https://github.com/bevyengine/bevy/issues/3776
            // so that wgpu does not clear the depth buffer.
            // As the opaque and alpha mask passes run first, opaque meshes can occlude
            // transparent ones.
            let depth_ops = Some(Operations {
                load: LoadOp::Load,
                store: true,
            });

            let mut render_pass = TrackedRenderPass::create_for_camera(
                render_context,
                "main_transparent_pass_3d",
                view_entity,
                target,
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
                Some(depth),
                depth_ops,
                &camera.viewport,
            );

            render_pass.render_phase(transparent_phase, world);
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(feature = "webgl")]
        if camera.viewport.is_some() {
            let _render_pass = TrackedRenderPass::create_for_camera(
                render_context,
                "reset_viewport_pass_3d",
                view_entity,
                target,
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
                None,
                None,
                &None,
            );
        }

        Ok(())
    }
}

use rustit_core::{ElementActivityLink, ElementActivityRole, Project};
use rustit_geometry::{GeometryKernel, Mesh, Point3, Segment3};
use rustit_geometry_truck::TruckGeometryKernel;
use rustit_ifc::ClassificationReference;
use rustit_model::{Level, Wall};
use rustit_schedule::Activity;
use std::{error::Error, sync::Arc};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

mod easter_eggs;
mod renderer;

use renderer::Renderer;

struct RustitApp {
    project: Project,
    wall_mesh: Mesh,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
}

impl RustitApp {
    fn demo() -> Result<Self, Box<dyn Error>> {
        let mut project = Project::new("The Royal Court");
        let level = Level::new("Dungeon Floor 1", 0.0);
        let mut wall = Wall::new(
            level.id,
            Segment3::new(Point3::new(0.0, 0.0, 0.0), Point3::new(6.0, 0.0, 0.0)),
            0.2,
            3.0,
        )?;
        wall.add_classification(ClassificationReference::uni_format(
            "project edition",
            "B2010",
            "Exterior Walls",
        )?);
        let mut activity = Activity::new("Carl constructs the first wall", 8);
        activity.add_classification(ClassificationReference::master_format(
            "project edition",
            "07 00 00",
            "Thermal and Moisture Protection",
        )?);
        let link = ElementActivityLink::new(wall.id, activity.id, ElementActivityRole::Construct);
        let wall_mesh = TruckGeometryKernel::default().wall_mesh(wall.geometry())?;

        project.model.add_level(level)?;
        project.model.add_wall(wall)?;
        project.schedule.add_activity(activity)?;
        project.link_element_activity(link)?;

        Ok(Self {
            project,
            wall_mesh,
            window: None,
            renderer: None,
        })
    }
}

impl ApplicationHandler for RustitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let title = format!(
                "Rustit — Achievement: It Opens | Project {} · Floor 1 · {} wall · {} activity · {} 4D link",
                easter_eggs::project_codename(self.project.id.as_uuid()),
                self.project.model.walls.len(),
                self.project.schedule.activities.len(),
                self.project.element_activity_links.len()
            );
            let attributes = Window::default_attributes()
                .with_title(title)
                .with_inner_size(LogicalSize::new(1100.0, 720.0));
            let window = Arc::new(
                event_loop
                    .create_window(attributes)
                    .expect("create Rustit window"),
            );
            let renderer = pollster::block_on(Renderer::new(window.clone(), &self.wall_mesh))
                .expect("initialize Rustit renderer");
            window.request_redraw();
            self.window = Some(window);
            self.renderer = Some(renderer);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(window.inner_size());
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };
                match renderer.render() {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        renderer.resize(window.inner_size());
                        window.request_redraw();
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Timeout) => window.request_redraw(),
                    Err(wgpu::SurfaceError::Other) => {}
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = RustitApp::demo()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_wall_mesh_is_generated_from_the_semantic_wall() {
        let app = RustitApp::demo().expect("demo app");
        let semantic_wall = &app.project.model.walls[0];
        let minimum = app.wall_mesh.vertices.iter().fold(
            Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            |extent, point| {
                Point3::new(
                    extent.x.min(point.x),
                    extent.y.min(point.y),
                    extent.z.min(point.z),
                )
            },
        );
        let maximum = app.wall_mesh.vertices.iter().fold(
            Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
            |extent, point| {
                Point3::new(
                    extent.x.max(point.x),
                    extent.y.max(point.y),
                    extent.z.max(point.z),
                )
            },
        );

        assert_eq!(semantic_wall.geometry().baseline.start.x, minimum.x);
        assert_eq!(app.project.name, "The Royal Court");
        assert_eq!(app.project.model.levels[0].name, "Dungeon Floor 1");
        assert_eq!(
            app.project.schedule.activities[0].name,
            "Carl constructs the first wall"
        );
        assert_eq!(semantic_wall.geometry().baseline.start.z, minimum.z);
        assert_eq!(minimum.y, -semantic_wall.thickness / 2.0);
        assert_eq!(maximum, Point3::new(6.0, 0.1, 3.0));
        assert_eq!(app.wall_mesh.triangles.len(), 12);
        let referenced = app
            .wall_mesh
            .triangles
            .iter()
            .flatten()
            .map(|index| app.wall_mesh.vertices[*index as usize])
            .collect::<Vec<_>>();
        assert!(referenced.iter().any(|point| point.x == 0.0));
        assert!(referenced.iter().any(|point| point.x == 6.0));
        assert!(referenced.iter().any(|point| point.z == 0.0));
        assert!(referenced.iter().any(|point| point.z == 3.0));
    }
}

use rustit_core::{ElementActivityLink, ElementActivityRole, Project};
use rustit_geometry::{Point3, Segment3};
use rustit_ifc::ClassificationReference;
use rustit_model::{Level, Wall};
use rustit_schedule::Activity;
use std::error::Error;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

mod easter_eggs;

struct RustitApp {
    project: Project,
    window: Option<Window>,
}

impl RustitApp {
    fn demo() -> Result<Self, Box<dyn Error>> {
        let mut project = Project::new("Untitled project");
        let level = Level::new("Level 1", 0.0);
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
        let mut activity = Activity::new("Construct first wall", 8);
        activity.add_classification(ClassificationReference::master_format(
            "project edition",
            "07 00 00",
            "Thermal and Moisture Protection",
        )?);
        let link = ElementActivityLink::new(wall.id, activity.id, ElementActivityRole::Construct);

        project.model.add_level(level)?;
        project.model.add_wall(wall)?;
        project.schedule.add_activity(activity)?;
        project.link_element_activity(link)?;

        Ok(Self {
            project,
            window: None,
        })
    }
}

impl ApplicationHandler for RustitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let title = format!(
                "Rustit — It Opens | Project {} · {} wall · {} activity · {} 4D link",
                easter_eggs::project_codename(self.project.id.as_uuid()),
                self.project.model.walls.len(),
                self.project.schedule.activities.len(),
                self.project.element_activity_links.len()
            );
            let attributes = Window::default_attributes()
                .with_title(title)
                .with_inner_size(LogicalSize::new(1100.0, 720.0));
            self.window = Some(
                event_loop
                    .create_window(attributes)
                    .expect("create Rustit window"),
            );
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if event == WindowEvent::CloseRequested {
            event_loop.exit();
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

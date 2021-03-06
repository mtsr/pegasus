#[macro_use] extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate pegasus;
extern crate specs;

// First, define the rendering pipeline

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

gfx_defines!{
    pipeline pipe {
        pos: gfx::Global<[f32; 2]> = "u_Pos",
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

const CODE_VS: &'static [u8] = b"
    #version 120
    uniform vec2 u_Pos;
    void main() {
        gl_Position = vec4(u_Pos, 0.0, 1.0);
    }
";

const CODE_FS: &'static [u8] = b"
    #version 120
    void main() {
        gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
    }
";

// Second, define a drawable component for our ECS
// also add a simple system to animate entities

struct Drawable([f32; 2]);

impl specs::Component for Drawable {
    type Storage = specs::VecStorage<Drawable>;
}

struct MoveSystem;

impl specs::System<pegasus::Delta> for MoveSystem {
    fn run(&mut self, arg: specs::RunArg, t: pegasus::Delta) {
        use specs::Join;
        let mut vis = arg.fetch(|w| w.write::<Drawable>());
        let angle = t;
        let (c, s) = (angle.cos(), angle.sin());
        for &mut Drawable(ref mut p) in (&mut vis).iter() {
            // rotation transform
            *p = [p[0]*c - p[1]*s, p[0]*s + p[1]*c];
        }
    }
}

// Third, define the program init logic for the ECS

struct Init;

impl pegasus::Init for Init {
    type Shell = ();
    fn start(self, plan: &mut pegasus::Planner) -> () {
        plan.add_system(MoveSystem, "move", 20);
        let mut w = plan.mut_world();
        use std::f32::consts::PI;
        let num = 200;
        for i in 0 .. num {
            let t = i as f32 / (num as f32);
            let angle = t * 7.0 * PI; 
            let pos = [t * angle.cos(), t * angle.sin()];
            w.create_now().with(Drawable(pos));
        }
    }
}

// Fourth, define the painter class to draw our entities

struct Painter<R: gfx::Resources> {
    slice: gfx::Slice<R>,
    pso: gfx::PipelineState<R, pipe::Meta>,
    data: pipe::Data<R>,
}

impl<R: gfx::Resources> pegasus::Painter<R> for Painter<R> {
    type Visual = Drawable;
    fn draw<'a, I, C>(&mut self, iter: I, enc: &mut gfx::Encoder<R, C>) where
        I: Iterator<Item = &'a Self::Visual>,
        C: gfx::CommandBuffer<R>
    {
        enc.clear(&self.data.out, [0.1, 0.2, 0.3, 1.0]);
        for &Drawable(pos) in iter {
            self.data.pos = pos.into();
            enc.draw(&self.slice, &self.pso, &self.data);
        }
    }
}

// Fifth, create the graphics context and instances of our new classes
// after that, we are free to fly

fn main() {
    use gfx::traits::FactoryExt;

    let builder = glutin::WindowBuilder::new()
        .with_title("Pegasus example".to_string())
        .with_dimensions(800, 600)
        .with_vsync();
    let (window, device, mut factory, main_color, _main_depth) =
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder);

    let init = Init;
    let painter = Painter {
        slice: gfx::Slice {
            start: 0,
            end: 1,
            base_vertex: 0,
            instances: None,
            buffer: gfx::IndexBuffer::Auto,
        },
        pso: {
            let prog = factory.link_program(CODE_VS, CODE_FS).unwrap();
            let rast = gfx::state::Rasterizer::new_fill();
            factory.create_pipeline_from_program(&prog,
                gfx::Primitive::PointList, rast, pipe::new()).unwrap()
        },
        data: pipe::Data {
            pos: [0.0, 0.0].into(),
            out: main_color,
        },
    };

    let mut pegasus = pegasus::Pegasus::new(init, device, painter, ||
        factory.create_command_buffer());
    
    'main: while let Some(_swing) = pegasus.swing() {
        window.swap_buffers().unwrap();
        for event in window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) |
                glutin::Event::Closed => break 'main,
                _ => (),
            }
        }
    }
}

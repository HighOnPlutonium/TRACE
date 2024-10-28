#[macro_use]
extern crate glium;
extern crate glutin_winit;
extern crate winit;

use std::any::Any;
use std::borrow::Cow;
use std::error::Error;
use std::iter;
use std::num::NonZeroU32;
use std::ops::{AddAssign, Deref};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use glium::{Blend, DepthTest, draw_parameters, DrawParameters, LinearBlendingFactor, Program, Surface, Texture2d, uniform, PolygonMode};
use glium::backend::{Facade, glutin::Display, glutin::glutin::context::ContextAttributesBuilder};
use glium::buffer::{Buffer, BufferMode, BufferType, Mapping};
use glium::glutin::config::ConfigTemplateBuilder;
use glium::glutin::context::Robustness;
use glium::glutin::display::{AsRawDisplay, GetGlDisplay};
use glium::glutin::prelude::{GlDisplay, NotCurrentGlContext};
use glium::glutin::surface::{Rect, SurfaceAttributesBuilder, WindowSurface};
use glium::index::{IndexBufferAny, PrimitiveType};
use glium::index::IndicesSource::NoIndices;
use glium::program::ComputeShader;
use glium::texture::{DepthTexture2d, MipmapsOption, RawImage2d, Texture2dArray, Texture2dDataSink, UncompressedFloatFormat};
use glium::texture::Dimensions::Texture3d;
use glium::uniforms::{ImageUnitAccess, ImageUnitFormat, MagnifySamplerFilter, MinifySamplerFilter, SamplerWrapFunction, UniformBuffer};
use glium::uniforms::UniformType::Image2d;
use glium::vertex::VertexBuffer;
use glium::vertex::VertexBufferAny;
use glutin_winit::DisplayBuilder;
use image::{ColorType, ImageFormat};
use itertools::Itertools;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};
use uuid::uuid;

mod util;




#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32;4],
}
implement_vertex!(Vertex,position);


fn make_kernel<F: Facade>(facade: &F, data: &[f32]) -> Result<Texture2dArray, Box<dyn Error>> {
    let size = data.len();
    let mut positive = Vec::<f32>::with_capacity(size);
    let mut negative = Vec::<f32>::with_capacity(size);

    data.iter().for_each(|x| {
        positive.push(f32::from(x.is_sign_positive()) * x);
        negative.push(f32::from(x.is_sign_negative()) * x);
    });
    let size = (size as f32).sqrt() as usize;
    let positive = positive.chunks_exact(size).map(|x|x.to_vec()).collect_vec();
    let negative = negative.chunks_exact(size).map(|x|x.to_vec()).collect_vec();
    Ok(Texture2dArray::with_format(facade, vec![positive,negative], UncompressedFloatFormat::F32, MipmapsOption::NoMipmap)?)
}

struct RenderStorage {
    vertex_buffer: VertexBufferAny,
    index_buffers: Vec<IndexBufferAny>,
    programs: Vec<Program>,
    computes: Vec<ComputeShader>,
    kernels: Vec<Texture2dArray>,
    previous: Texture2d,
    stuff: f32,
    history: Vec<Texture2d>
}
impl RenderStorage {
    fn next(&mut self, next: &Texture2d) {
        next.as_surface().fill(&self.previous.as_surface(), MagnifySamplerFilter::Nearest);
    }
    fn more(&mut self, other: f32) {
        self.stuff += other;
    }
    fn less(&mut self, other: f32) {
        self.stuff -= other;
    }
}
struct TimedStorage {
    process: Box<Instant>,
    disk_write: Box<Instant>,
}

#[derive(Default)]
struct App {
    window: Option<Window>,
    display: Option<Display<WindowSurface>>,
    storage: Option<RenderStorage>,
    instant: Option<TimedStorage>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {

        if self.window.is_none() || self.display.is_none() {

            (self.window, self.display) = demiurge(event_loop);
            self.instant = Some(TimedStorage {
                process:    Box::new(Instant::now()),
                disk_write: Box::new(Instant::now()),
            })
        }
        if let Some(_display) = self.display.as_ref() {

            //currently empty. will be trying to get text display to work in here
        }

        //manually call DroppedFile-event. used to display something as a "default".
        if !(std::env::consts::OS.eq("macos")|std::env::consts::OS.eq("linux")) {
        self.window_event(event_loop,
                          self.window.as_ref().unwrap().id(),
                          WindowEvent::DroppedFile(
                                PathBuf::from(r".\src\resources\4\tesseract.obj")));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {

            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event: event::KeyEvent {
                    state: ElementState::Pressed,
                    logical_key: Key::Named(NamedKey::Escape),
                    .. }, .. } => { event_loop.exit() },

            WindowEvent::Resized(size) => {
                self.display.as_ref().unwrap().resize(size.into());
                self.window.as_ref().unwrap().request_redraw();
            },

            WindowEvent::DroppedFile(path) => {
                if let Some(display) = self.display.as_ref() {

                    let (mut vertex_buffer, index_buffer) = util::wavefront(display, PrimitiveType::TriangleStrip, path.clone());
                    let (_, wireframe_buffer) = util::wavefront(display, PrimitiveType::LineStrip, path.clone());
                    let mut programs: Vec<Program> = Vec::with_capacity(3);
                    let mut computes: Vec<ComputeShader> = Vec::with_capacity(1);
                    let mut index_buffers: Vec<IndexBufferAny> = Vec::with_capacity(2);
                    //the index buffer normally used for the object
                    index_buffers.push(index_buffer);
                    //the index buffer used for the wireframe display
                    index_buffers.push(wireframe_buffer);
                    //program containing the "default" vertex shader
                    // + somewhat modified blinn-phong shading as the fragment shader
                    programs.push(Program::from_source(display,
                                                       &*util::load_shader(util::VERT_SHADER_SRC),
                                                       &*util::load_shader(util::FRAG_SHADER_SRC),
                                                       None).unwrap());
                    //same as above but with a different fragment shader. originally used for shading faces according to their normals
                    programs.push(Program::from_source(display,
                                                       &*util::load_shader(util::VERT_SHADER_SRC),
                                                       &*util::load_shader(util::FRAG_SHADER_NRM),
                                                       None).unwrap());
                    //the vertex and fragment shader used to display a previously drawn texture.
                    // AS OF NOW, THIS IS UNNECESSARY!!! YOU CAN SIMPLY FILL A TEXTURE INTO THE BACK BUFFER WITHOUT THIS INTERMEDIATE
                    programs.push(Program::from_source(display,
                                                       &*util::load_shader(util::VERT_SHADER_BUF),
                                                       &*util::load_shader(util::FRAG_SHADER_BUF),
                                                       None).unwrap());
                    //an unused debug compute shader. simply here for some place to test out some stuff, if needed.
                    computes.push(ComputeShader::from_source(display,
                                                             &*util::load_shader(util::COMP_SHADER_DBG)
                    ).unwrap());
                    //the compute shader used for the "framebuffer convolution". the only thing inside this Vec<> rn.
                    computes.push(ComputeShader::from_source(display,
                                                             &*util::load_shader(util::COMP_SHADER_CNV)
                                                             ).unwrap());
                    let step = 0.02;
                    let x = 1.0f32;
                    let y = 1.0f32;
                    let z = 1.0f32;
                    let w = 1.0f32;
                    let size = x*y*z*w/step;
                    let mut data: Vec<Vertex> = Vec::with_capacity(size as usize);
                    let mut vertpos = [0.0,0.0,0.0,0.0f32];
                    for a in 0..(x/step) as usize {
                        vertpos[0] = (a as f32)*step-x/2.0;
                        for b in 0..(y/step) as usize {
                            vertpos[1] = (b as f32)*step-y/2.0;
                            for c in 0..(z/step) as usize {
                                vertpos[2] = (c as f32)*step-z/2.0;
                                for d in 0..(w/step) as usize {
                                    vertpos[3] = (d as f32)*step-w/2.0;
                                    data.push(Vertex{position:vertpos});
                                }
                            }
                        }
                    }
                    let grid_vbuf = VertexBuffer::new(display, &data).unwrap();
                    vertex_buffer = grid_vbuf.into();
                    let mut kernel_data: Vec<_> = Vec::with_capacity(6);
                    kernel_data.push([
                        [0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0],
                    ].into_iter().flatten().map(|x| x / 1f32).collect_vec());
                    kernel_data.push([
                        [ 0.0,-1.0, 0.0],
                        [-1.0, 4.0,-1.0],
                        [ 0.0,-1.0, 0.0],
                    ].into_iter().flatten().map(|x|x/1f32).collect_vec());
                    kernel_data.push([
                        [-1.0,-1.0,-1.0],
                        [-1.0, 8.0,-1.0],
                        [-1.0,-1.0,-1.0],
                    ].into_iter().flatten().map(|x|x/1f32).collect_vec());
                    kernel_data.push([
                        [ 1.01, 1.0, 0.0],
                        [ 1.0, 0.1,-1.0],
                        [ 0.0,-1.0,-1.0],
                    ].into_iter().flatten().map(|x|x/1f32).collect_vec());
                    kernel_data.push([
                        [ 1.0,  4.0,  6.0,  4.0, 1.0],
                        [ 4.0, 16.0, 24.0, 16.0, 4.0],
                        [ 6.0, 24.0, 36.0, 24.0, 6.0],
                        [ 4.0, 16.0, 24.0, 16.0, 4.0],
                        [ 1.0,  4.0,  6.0,  4.0, 1.0],
                    ].into_iter().flatten().map(|x|x/256f32).collect_vec());
                    kernel_data.push([
                        [ 1.0,  4.0,   6.0,  4.0, 1.0],
                        [ 4.0, 16.0,  24.0, 16.0, 4.0],
                        [ 6.0, 24.0,-476.0, 24.0, 6.0],
                        [ 4.0, 16.0,  24.0, 16.0, 4.0],
                        [ 1.0,  4.0,   6.0,  4.0, 1.0],
                    ].into_iter().flatten().map(|x|x/-256f32).collect_vec());
                    kernel_data.push([
                        [0.25, 0.5, 0.75, 1.0, 1.0, 1.0, 0.75, 0.5, 0.25],
                        [0.5, 0.25, 0.5, 0.75, 0.75, 0.75, 0.5, 0.25, 0.5],
                        [0.75, 0.5, 0.25, 0.5, 0.5, 0.5, 0.25, 0.5, 0.75],
                        [1.0, 0.75, 0.5, 0.25, 0.25, 0.25, 0.5, 0.75, 1.0],
                        [1.0, 0.75, 0.5, 0.25, -99.0, 0.25, 0.5, 0.75, 1.0],
                        [1.0, 0.75, 0.5, 0.25, 0.25, 0.25, 0.5, 0.75, 1.0],
                        [0.75, 0.5, 0.25, 0.5, 0.5, 0.5, 0.25, 0.5, 0.75],
                        [0.5, 0.25, 0.5, 0.75, 0.75, 0.75, 0.5, 0.25, 0.5],
                        [0.25, 0.5, 0.75, 1.0, 1.0, 1.0, 0.75, 0.5, 0.25f32],
                    ].into_iter().flatten().map(|x|x/-56f32).collect_vec());
                    let kernels: Vec<Texture2dArray> = kernel_data.iter().map(|x|make_kernel(display,x).unwrap()).collect_vec();
                    let previous = Texture2d::empty(display,display.get_framebuffer_dimensions().0,display.get_framebuffer_dimensions().1).unwrap();
                    previous.as_surface().clear_color(0f32,0f32,0f32,0f32);
                    let stuff = 0f32;
                    let history: Vec<Texture2d> = Vec::with_capacity(60);
                    self.storage = Some(RenderStorage {
                        vertex_buffer,
                        index_buffers,
                        programs,
                        computes,
                        kernels,
                        previous,
                        stuff,
                        history,
                    });
                    self.window.as_ref().unwrap().request_redraw();
                }},

            WindowEvent::RedrawRequested => {
                if let (Some(display),Some(window)) = (self.display.as_ref(),self.window.as_ref()) {
                    let mut frame = display.draw();
                    let (width, height) = display.get_framebuffer_dimensions();
                    frame.clear_color_and_depth((0.0,0.0,0.0,0.0), 1.0);

                    //naming of stuff here is god-awful. tex is here to contain the result of the compute shader calls.
                    let tex = Texture2d::empty_with_format(display,
                                                               UncompressedFloatFormat::U8U8U8U8,
                                                               MipmapsOption::NoMipmap,
                                                               width,height).unwrap();
                    tex.as_surface().clear_color(0.0,0.0,0.0,0.0);
                    let tex = tex.sampled()
                        .magnify_filter(MagnifySamplerFilter::Nearest)
                        .minify_filter(MinifySamplerFilter::Nearest)
                        .wrap_function(SamplerWrapFunction::Clamp).0;

                    //same as above, only that this contains the result of the first drawing pass
                    let alt_tex = Texture2d::empty_with_format(display,
                                                               UncompressedFloatFormat::U8U8U8U8,
                                                               MipmapsOption::NoMipmap,
                                                               width,height).unwrap();
                    alt_tex.as_surface().clear_color(0.0,0.0,0.0,0.0);
                    let alt_tex = alt_tex.sampled()
                        .magnify_filter(MagnifySamplerFilter::Nearest)
                        .minify_filter(MinifySamplerFilter::Nearest)
                        .wrap_function(SamplerWrapFunction::Clamp).0;

                    //I don't really know if these condition bindings even make sense. they work tho
                    if let (
                        Some(storage),
                        Some(TimedStorage{process: ref mut process_time, disk_write: ref mut last_write }),
                        ) = (self.storage.as_mut(),self.instant.as_mut()) {

                        //some of the code here is pretty old. some parts have been overhauled but most of it is either unused or not working.
                        let t = process_time.elapsed().as_secs_f32();
                        let aspect_ratio = (|xy: PhysicalSize<f32>|{xy.height/xy.width})(window.inner_size().cast::<f32>());

                        let scale = 1.0f32;
                        let t2 = t / 3.0;
                        let t3 = t / 2.0;

                        let uniforms = uniform! {
                            resolution:
                                (|xy: PhysicalSize<u32>|{[xy.width,xy.height]})(window.inner_size()),
                            time:
                                t,
                            xz_yw_rot:
                               [[ 1.0,      0.0,        0.0,        0.0     ],
                                [ 0.0,      t3.cos(),   0.0,       -t3.sin()],
                                [ 0.0,      0.0,        1.0,        0.0     ],
                                [ 0.0,      t3.sin(),   0.0,        t3.cos()]],
                            yz_xw_rot:
                               [[ t3.cos(), 0.0,        0.0,       -t3.sin()],
                                [ 0.0,      1.0,        0.0,        0.0     ],
                                [ 0.0,      0.0,        1.0,        0.0     ],
                                [ t3.sin(), 0.0,        0.0,        t3.cos()]],
                            xy_zw_rot:
                               [[ 1.0,      0.0,        0.0,        0.0     ],
                                [ 0.0,      1.0,        0.0,        0.0     ],
                                [ 0.0,      0.0,        t3.cos(),  -t3.sin()],
                                [ 0.0,      0.0,        t3.sin(),   t3.cos()]],
                            proj_point:
                                [ 0.0,      0.0,        0.0,        1.5f32],
                            u_light:
                                [ 1.4,     -0.4,        0.7f32],
                            scale:
                               [[ scale*aspect_ratio,   0.0,        0.0,        0.0   ],
                                [ 0.0,                  scale,      0.0,        0.0   ],
                                [ 0.0,                  0.0,        scale,      0.0   ],
                                [ 0.0,                  0.0,        0.0,        1.0f32]],
                            speen:
                               [[ t2.cos().powi(2),                            -t2.cos()*t2.sin(),                              t2.sin(),           0.0   ],
                                [ t2.sin().powi(2)*t2.cos()+t2.cos()*t2.sin(),  t2.cos().powi(2)-t2.sin().powi(3),             -t2.cos()*t2.sin(),  0.0   ],
                                [-t2.cos().powi(2)*t2.sin()+t2.sin().powi(2),   t2.sin().powi(2)*t2.cos()+t2.cos()*t2.sin(),    t2.cos().powi(2),   0.0   ],
                                [ 0.0,                                          0.0,                                            0.0,                1.0f32]],
                            };


                        //got two different blending functions here. both of em aren't really optimal for what I want, but a workaround would be both unstable and annoying
                        let mut blend_nrm = Blend::alpha_blending();
                        blend_nrm.constant_value = (0.0,0.0,0.0,0.7);
                        blend_nrm.alpha = draw_parameters::BlendingFunction::Subtraction
                            { source: LinearBlendingFactor::SourceColor, destination: LinearBlendingFactor::SourceColor };
                        blend_nrm.color = draw_parameters::BlendingFunction::Addition
                            { source: LinearBlendingFactor::SourceAlphaSaturate, destination: LinearBlendingFactor::ConstantAlpha };

                        let mut blend_src = Blend::alpha_blending();
                        blend_src.alpha = draw_parameters::BlendingFunction::Addition
                            { source: LinearBlendingFactor::One, destination: LinearBlendingFactor::One };
                        blend_src.color = draw_parameters::BlendingFunction::Addition
                            { source: LinearBlendingFactor::SourceAlpha, destination: LinearBlendingFactor::SourceAlpha };



                        let mut blend_dbg = Blend::alpha_blending();
                        blend_dbg.alpha = draw_parameters::BlendingFunction::Addition
                        { source: LinearBlendingFactor::One, destination: LinearBlendingFactor::One };
                        blend_dbg.color = draw_parameters::BlendingFunction::Addition
                        { source: LinearBlendingFactor::One, destination: LinearBlendingFactor::One };


                        let texture1 = Texture2d::empty(display,width,height).unwrap();
                        let texture2 = Texture2d::empty(display,width,height).unwrap();
                        let output = [ ("output1", &texture1), ("output2", &texture2) ];
                        let depth = DepthTexture2d::empty(display,width,height).unwrap();
                        let mut multi = glium::framebuffer::MultiOutputFrameBuffer::with_depth_buffer(display, output.iter().cloned(),&depth).unwrap();
                        multi.clear_color_and_depth((1.0,0.0,0.0,0.0),0.0);
                        let params = DrawParameters {
                            depth: glium::Depth {
                                test: DepthTest::IfLess,
                                write: false,
                                ..Default::default()},
                            blend: blend_dbg,
                            point_size: Some(1.0),
                            line_width: Some(2.0),
                            polygon_mode: PolygonMode::Fill,
                            ..Default::default()};
                        multi.draw(
                            &storage.vertex_buffer,
                            NoIndices { primitives: PrimitiveType::Points },
                            &storage.programs[1],
                            &uniforms,
                            &params).unwrap();
                        multi.color_attachments;
                        //texture2.as_surface().fill(&frame, MagnifySamplerFilter::Nearest);

                        /*let params = DrawParameters {
                            depth: glium::Depth {
                                test: DepthTest::IfLess,
                                write: false,
                                ..Default::default()},
                            blend: blend_nrm,
                            primitive_restart_index: true,
                            ..Default::default()};
                        frame.draw(
                            &storage.vertex_buffer,
                            &storage.index_buffers[0],
                            &storage.programs[1],
                            &uniforms,
                            &params).unwrap();*/



                        frame.fill(&alt_tex.as_surface(), MagnifySamplerFilter::Nearest);/*
                        frame.clear_color_and_depth((0.0,0.0,0.0,0.0), 1.0);

                        let empty = Texture2d::empty(display,width,height).unwrap();
                        empty.as_surface().clear_color(0.0,0.0,0.0,0.0);
                        let image_unit = tex
                            .image_unit(ImageUnitFormat::RGBA8).unwrap()
                            .set_access(ImageUnitAccess::Read);
                        let _ = &storage.computes[1].execute(   //COMPUTE SHADER STUFF!!! YAY!!!
                            uniform! {
                                kernel:     &storage.kernels[0],
                                previous:   storage.history.get(1).unwrap_or(&empty),
                                time:       t,
                                width:      tex.width(),
                                height:     tex.height(),
                                dst:        image_unit,
                                src:        alt_tex,
                            },  tex.width(),tex.height(),1);

                        /*if storage.stuff > 0f32  {
                            storage.next(alt_tex);
                            storage.less(2f32);
                        }
                        storage.more(1f32);*/
                        if storage.history.len() < 3 {
                            storage.history.push(Texture2d::new(display,tex.read_to_pixel_buffer().read_as_texture_2d::<RawImage2d<u8>>().unwrap()).unwrap());
                        } else {
                            storage.history.remove(0);
                        }
                        tex.as_surface().fill(&frame, MagnifySamplerFilter::Nearest);*/



                        //commented the below stuff to enable/disable as needed during development.
                        //as such, enabling this is effectively hardcoded. do I need to explain why this is "suboptimal"?
                        //uncomment for an additional render pass that draws a wireframe
                        /*
                        let params = DrawParameters {
                            blend: Blend::alpha_blending(),
                            primitive_restart_index: true,
                            ..Default::default()};
                        frame.draw(
                            &storage.vertex_buffer,
                            &storage.index_buffers[1],
                            &storage.programs[0],
                            &uniforms,
                            &params).unwrap();
                         */
                        //display.flush();

                        /*tex.as_surface().clear_color(0.0,0.0,0.0,0.0f32);
                        let image_unit = tex
                            .image_unit(ImageUnitFormat::RGBA8).unwrap()
                            .set_access(ImageUnitAccess::ReadWrite);
                        let _ = &storage.computes[0].execute(
                                                                uniform! {
                                time:       t,
                                width:      tex.width(),
                                height:     tex.height(),
                                dst:        image_unit,
                            },  tex.width(),tex.height(),1);
                        tex.as_surface().fill(&frame, MagnifySamplerFilter::Nearest);*/

                        //the "save-to-file" functionality doesn't properly work rn.
                        //using something like this "do_save" variable is bad. needed for easier debugging.
                        let do_save: bool = false;
                        if last_write.elapsed() >= Duration::from_secs(2) && do_save {

                            let elapsed = yoink_to_file(display, r"c:/users/joelk/desktop/out.png").unwrap();

                            process_time.add_assign(elapsed);
                            **last_write = Instant::now();
                            println!("      {:?}",process_time.elapsed());
                        }


                    }
                    frame.finish().unwrap();
                }
                self.window.as_ref().unwrap().request_redraw(); //interesting stuff in "request_redraw" docstring
            },
            _ => ()
        }
    }
}


fn main()
{
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    _ = event_loop.run_app(&mut app);
}

    //it does what it says it does. other than the line using the Cow<> CloneOnWrite pointer.
    fn yoink_to_file<P: AsRef<Path>>
    (display: &Display<WindowSurface>, path: P) -> Result<Duration, Box<dyn Error>> {

        let function_instant = Instant::now();
        let (width,height) = display.get_framebuffer_dimensions();

        let buf: Cow<[u8]> = display.read_front_buffer::<RawImage2d<u8>>()?.data;
        //let buf: Cow<[u8]> = Cow::from(&[u8::MAX;1920000]);
        image::save_buffer_with_format(path, &buf, width, height, ColorType::Rgba8, ImageFormat::Png)?;
        Ok(function_instant.elapsed())
    }


    //at the time when I created this part, glium hadn't properly updated their own code, so I copied their code from their library
    //and modified + shortened it so that it actually works the way I needed/wanted it to. maybe they've changed stuff again, and it would work without this
    //but I cant be bothered to check and/or fix everything again. at least right now.
    pub fn demiurge(event_loop: &ActiveEventLoop) -> (Option<Window>, Option<Display<WindowSurface>>)
    {
        let window_attributes = WindowAttributes::default()
            .with_transparent(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_decorations(true)
            .with_title("TRACE");
        let display_builder = DisplayBuilder::new()
                .with_window_attributes(Some(window_attributes));
        let config_template_builder = ConfigTemplateBuilder::new();

        let (window, gl_config) = display_builder.build(
            event_loop,
            config_template_builder,
            |mut configs| configs.next().unwrap()
        ).unwrap();

        let window = window.unwrap();

        let (width, height): (u32, u32) = window.inner_size().into();

        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.window_handle().unwrap().into(),
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap()
        );

        let surface = unsafe { gl_config
                .display()
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };
        let context_attributes = ContextAttributesBuilder::new()
            .with_debug(false).with_robustness(Robustness::NotRobust)
            .build(Some(window.window_handle().unwrap().into())
            );
        let current_context = unsafe { gl_config
            .display()
            .create_context(&gl_config, &context_attributes)
            .unwrap()
        }.make_current(&surface).unwrap();
        let display = Display::from_context_surface(current_context, surface).unwrap();

        (Some(window), Some(display))
    }
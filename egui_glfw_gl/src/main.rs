use egui_backend::EguiInputState;
use image;

use egui_backend::egui::{self, vec2, Pos2, Rect};
use egui_glfw_gl as egui_backend;
use egui_glfw_gl::glfw::{Context, Key};

use glfw::{Action, Modifiers};

use gl;
use gl::types::*;

use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use std::{ffi::c_void, mem, ptr};

mod shader;
use shader::Shader;

const SCR_WIDTH: u32 = 800;
const SCR_HEIGHT: u32 = 600;

static mut TOGGLE_GUI: bool = true;
static mut RECOMPILE_SHADERS: bool = false;

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));

    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    let (mut window, events) = glfw
        .create_window(
            SCR_WIDTH,
            SCR_HEIGHT,
            "learn-opengl-rs",
            glfw::WindowMode::Windowed,
        )
        .expect("Failed to create window");

    window.make_current();
    window.set_all_polling(true);

    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let mut shader = Shader::new(
        "resources/shaders/vertex.vs",
        "resources/shaders/fragment.fs",
    );

    #[rustfmt::skip]
    let rectangle: [f32; 48] = [
        // positions          // colors           // texture coords
         0.5,  0.5, 0.0,   1.0, 0.0, 0.0,   1.0, 1.0,   // top right
         0.5, -0.5, 0.0,   0.0, 1.0, 0.0,   1.0, 0.0,   // bottom right
        -0.5, -0.5, 0.0,   0.0, 0.0, 1.0,   0.0, 0.0,   // bottom left

         0.5,  0.5, 0.0,   1.0, 0.0, 0.0,   1.0, 1.0,    // top right 
        -0.5,  0.5, 0.0,   0.0, 1.0, 0.0,   0.0, 1.0,    // top left 
        -0.5, -0.5, 0.0,   0.0, 0.0, 1.0,   0.0, 0.0,    // bottom left 
    ];

    let (mut vbo, mut vao) = ([0; 1], [0; 1]);
    let mut texture = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao as *mut _);
        gl::GenBuffers(1, &mut vbo as *mut _);

        gl::BindVertexArray(vao[0]);

        gl::BindBuffer(gl::ARRAY_BUFFER, vbo[0]);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (rectangle.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            &rectangle[0] as *const f32 as *const c_void,
            gl::STATIC_DRAW,
        );

        let stride = 8 * mem::size_of::<GLfloat>() as GLsizei;

        // Position
        gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());
        gl::EnableVertexAttribArray(0);

        // Color
        gl::VertexAttribPointer(
            1,
            3,
            gl::FLOAT,
            gl::FALSE,
            stride,
            (3 * mem::size_of::<GLfloat>()) as *mut _,
        );
        gl::EnableVertexAttribArray(1);

        // Texture
        gl::VertexAttribPointer(
            2,
            2,
            gl::FLOAT,
            gl::FALSE,
            stride,
            (6 * mem::size_of::<GLfloat>()) as *const c_void,
        );
        gl::EnableVertexAttribArray(2);

        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        let img = image::open(&Path::new("resources/textures/container.jpg"))
            .expect("Failed to load texture");
        let data: Vec<u8> = img.to_rgb8().into_raw();
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGB as i32,
            img.width() as i32,
            img.height() as i32,
            0,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            &data[0] as *const u8 as *const c_void,
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);
    }

    let mut painter = egui_backend::Painter::new(&mut window, SCR_WIDTH, SCR_HEIGHT);
    let mut egui_ctx = egui::CtxRef::default();

    let (width, height) = window.get_framebuffer_size();
    let native_pixels_per_point = window.get_content_scale().0;

    let mut egui_input_state = egui_backend::EguiInputState::new(egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            Pos2::new(0f32, 0f32),
            vec2(width as f32, height as f32) / native_pixels_per_point,
        )),
        pixels_per_point: Some(native_pixels_per_point),
        ..Default::default()
    });
    let start_time = Instant::now();

    while !window.should_close() {
        egui_input_state.input.time = Some(start_time.elapsed().as_secs_f64());
        egui_ctx.begin_frame(egui_input_state.input.take());

        //In egui 0.10.0 we seem to be losing the value to pixels_per_point,
        //so setting it every frame now.
        egui_input_state.input.pixels_per_point = Some(native_pixels_per_point);

        process_events(&mut window, &events, &mut egui_input_state);

        if unsafe { RECOMPILE_SHADERS } {
            shader = Shader::new(
                "resources/shaders/vertex.vs",
                "resources/shaders/fragment.fs",
            );
            unsafe {
                RECOMPILE_SHADERS = false;
            }
        }

        clear_viewport();

        render(shader, vao[0], &glfw, texture);

        if unsafe { TOGGLE_GUI } {
            render_gui(&egui_ctx, &mut egui_input_state, &mut painter);
        }

        window.swap_buffers();

        glfw.poll_events();
    }
}

fn render_gui(
    egui_ctx: &egui::CtxRef,
    egui_input_state: &mut egui_backend::EguiInputState,
    painter: &mut egui_backend::Painter,
) {
    egui::SidePanel::left("side_panel").show(&egui_ctx, |ui| {
        ui.heading("Side Panel");

        if ui.button("Recompile Shader").clicked() {
            unsafe {
                RECOMPILE_SHADERS = true;
            }
        };

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |_ui| {
            // Side panel Footer
        });
    });

    let (egui_output, paint_cmds) = egui_ctx.end_frame();

    if !egui_output.copied_text.is_empty() {
        egui_backend::copy_to_clipboard(egui_input_state, egui_output.copied_text);
    }

    let paint_jobs = egui_ctx.tessellate(paint_cmds);

    painter.paint_jobs(
        None,
        paint_jobs,
        &egui_ctx.texture(),
        egui_input_state.input.pixels_per_point.unwrap(),
    );
}

fn process_events(
    window: &mut glfw::Window,
    events: &Receiver<(f64, glfw::WindowEvent)>,
    egui_input_state: &mut EguiInputState,
) {
    for (_, event) in glfw::flush_messages(events) {
        match event {
            glfw::WindowEvent::FramebufferSize(width, height) => unsafe {
                gl::Viewport(0, 0, width, height)
            },
            glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                window.set_should_close(true)
            }
            glfw::WindowEvent::Key(Key::H, _, Action::Press, Modifiers::Control) => unsafe {
                TOGGLE_GUI = !TOGGLE_GUI;
            },
            _ => {
                egui_backend::handle_event(event, egui_input_state);
            }
        }
    }
}

fn clear_viewport() {
    unsafe {
        gl::ClearColor(21.0 / 255.0, 43.0 / 255.0, 60.0 / 255.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
}

fn render(shader: Shader, vao: u32, _glfw: &glfw::Glfw, texture: u32) {
    unsafe {
        shader.use_program();

        gl::BindTexture(gl::TEXTURE_2D, texture);

        gl::BindVertexArray(vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
    }
}

use image;
use imgui::Context as ImContext;
use imgui_glfw_rs::glfw::{self, Action, Context, Key, Modifiers};
use imgui_glfw_rs::imgui::ImString;
use imgui_glfw_rs::{imgui, ImguiGLFW};

use gl;
use gl::types::*;

use std::path::Path;
use std::sync::mpsc::Receiver;
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
            "imgui-rs_glfw_gl",
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

    let mut imgui = ImContext::create();

    let mut imgui_glfw = ImguiGLFW::new(&mut imgui, &mut window);

    while !window.should_close() {
        process_events(&mut window, &events, &mut imgui, &mut imgui_glfw);

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

        if unsafe { TOGGLE_GUI } {
            render_gui(&mut window, &mut imgui, &mut imgui_glfw);
        }

        render(shader, vao[0], glfw, texture);

        window.swap_buffers();

        glfw.poll_events();
    }
}

fn process_events(
    window: &mut glfw::Window,
    events: &Receiver<(f64, glfw::WindowEvent)>,
    imgui: &mut ImContext,
    imgui_glfw: &mut ImguiGLFW,
) {
    for (_, event) in glfw::flush_messages(events) {
        imgui_glfw.handle_event(imgui, &event);
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
            _ => {}
        }
    }
}

fn clear_viewport() {
    unsafe {
        gl::ClearColor(21.0 / 255.0, 43.0 / 255.0, 60.0 / 255.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }
}

fn render(shader: Shader, vao: u32, _glfw: glfw::Glfw, texture: u32) {
    unsafe {
        shader.use_program();

        gl::BindTexture(gl::TEXTURE_2D, texture);

        gl::BindVertexArray(vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
    }
}

fn render_gui(window: &mut glfw::Window, imgui: &mut ImContext, imgui_glfw: &mut ImguiGLFW) {
    let ui = imgui_glfw.frame(window, imgui);

    // ui.show_demo_window(&mut true);
    ui.window(&ImString::new("Controls"))
        .size([200.0, 200.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.tree_node(&ImString::new("Shaders")).build(|| {
                if ui.button(&ImString::new("Recompile Shader"), [180.0, 20.0]) {
                    unsafe {
                        RECOMPILE_SHADERS = true;
                    }
                }
            });
        });

    imgui_glfw.draw(ui, window);
}

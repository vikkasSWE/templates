extern crate gl;
use std::{ffi::CString, fs::File, io::Read, ptr};

use self::gl::types::*;

#[derive(Debug, Clone, Copy)]
pub struct Shader {
    pub id: u32,
}

impl Shader {
    pub fn new(vertex_path: &str, fragment_path: &str) -> Shader {
        let mut shader = Shader { id: 0 };

        let mut vertex_shader_file =
            File::open(vertex_path).unwrap_or_else(|_| panic!("Failed to open {}", vertex_path));

        let mut fragment_shader_file = File::open(fragment_path)
            .unwrap_or_else(|_| panic!("Failed to open {}", fragment_path));

        let mut vertex_code = String::new();
        let mut fragment_code = String::new();

        vertex_shader_file
            .read_to_string(&mut vertex_code)
            .expect("Failed to read vertex shader");

        fragment_shader_file
            .read_to_string(&mut fragment_code)
            .expect("Failed to read vertex shader");

        let vertex_shader_code = CString::new(vertex_code.as_bytes()).unwrap();
        let fragment_shader_code = CString::new(fragment_code.as_bytes()).unwrap();

        unsafe {
            let vertex = gl::CreateShader(gl::VERTEX_SHADER);
            gl::ShaderSource(vertex, 1, &vertex_shader_code.as_ptr(), ptr::null());
            gl::CompileShader(vertex);
            shader.check_compile_errors(vertex, "VERTEX");

            let fragment = gl::CreateShader(gl::FRAGMENT_SHADER);
            gl::ShaderSource(fragment, 1, &fragment_shader_code.as_ptr(), ptr::null());
            gl::CompileShader(fragment);
            shader.check_compile_errors(fragment, "FRAGMENT");

            let shader_program = gl::CreateProgram();
            gl::AttachShader(shader_program, vertex);
            gl::AttachShader(shader_program, fragment);
            gl::LinkProgram(shader_program);
            shader.check_compile_errors(shader_program, "PROGRAM");

            gl::DeleteShader(vertex);
            gl::DeleteShader(fragment);

            shader.id = shader_program;

            shader
        }
    }

    unsafe fn check_compile_errors(&self, shader: u32, type_: &str) {
        let mut success = gl::FALSE as GLint;
        let mut info_log = Vec::with_capacity(1024);

        info_log.set_len(1024 - 1); // subtract 1 to skip the trailing null character
        if type_ != "PROGRAM" {
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                gl::GetShaderInfoLog(
                    shader,
                    1024,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                println!(
                    "ERROR::SHADER::{}::COMPILATION_FAILED\n{}\n",
                    type_,
                    std::str::from_utf8(&info_log).unwrap()
                );
            }
        } else {
            gl::GetProgramiv(shader, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                gl::GetProgramInfoLog(
                    shader,
                    1024,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                println!(
                    "ERROR::SHADER::PROGRAM::LINKING_FAILED\n{}\n",
                    std::str::from_utf8(&info_log).unwrap()
                );
            }
        }
    }

    pub unsafe fn use_program(&self) {
        gl::UseProgram(self.id);
    }

    pub unsafe fn set_f32(&self, name: &str, value: f32) {
        let uniform = CString::new(name).unwrap();
        gl::Uniform1f(
            gl::GetUniformLocation(self.id, uniform.as_c_str().as_ptr()),
            value,
        );
    }
}

extern crate sdl2;

#[allow(non_upper_case_globals)]
mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use std::ffi::CString;
use std::mem;
use std::ptr;
use std::str;

use gl::types::*;

use sdl2::video::GLProfile;

// Constants
const NUM_VECS: usize = 10;

// Structure definitions
struct PosVec {
    x: f32,
    y: f32,
    z: f32,
}

// Shader sources
static CS_SRC: &'static str =
   "#version 450 core

    layout(local_size_x=10) in;

    layout (packed, binding = 0) buffer Pos
    {
        float[10][3] Positions;
    };

    void main()
    {
        uint gid = gl_GlobalInvocationID.x;
        Positions[gid][0] = gid;
        Positions[gid][1] = gid;
        Positions[gid][2] = gid;
        memoryBarrier();
    }
    ";

fn get_program_from_shader(src: &str, ty: GLenum) -> GLuint {
    let program;

    unsafe {
        let shader = gl::CreateShader(ty);

        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);

            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);

            panic!("Compile error: {}", str::from_utf8(&buf).ok().expect("ShaderInfoLog not valid utf8"));
        }

        program = gl::CreateProgram();
        gl::AttachShader(program, shader);
        gl::LinkProgram(program);

        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);

            let mut buf = Vec::with_capacity(len as usize - 1);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            
            panic!("Link error: {}", str::from_utf8(&buf).ok().expect("ProgramInfoLog not valid utf8"));
        }
    } 

    program
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let gl_attr = video_subsystem.gl_attr();

    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_flags().forward_compatible().set();
    gl_attr.set_context_version(4, 5);

    let window = video_subsystem.window("Window", 800, 600)
        .resizable()
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let _ctx = window.gl_create_context();
    gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

    // Compile shaders
    let program = get_program_from_shader(CS_SRC, gl::COMPUTE_SHADER);
    let mut pos_ssbo: GLuint = 0;

    // Setup the shader storage buffer
    unsafe {
        gl::GenBuffers(1, &mut pos_ssbo);
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 0, pos_ssbo);
        gl::BufferData(gl::SHADER_STORAGE_BUFFER, (NUM_VECS * mem::size_of::<PosVec>()) as isize, ptr::null(), gl::STATIC_DRAW);

        let positions = gl::MapBufferRange(
            gl::SHADER_STORAGE_BUFFER, 0, (NUM_VECS * mem::size_of::<PosVec>()) as isize, gl::MAP_WRITE_BIT
        ) as *mut [PosVec; NUM_VECS];

        for idx in 0..NUM_VECS {
            (*positions)[idx].x = 0.0;
            (*positions)[idx].y = 0.0;
            (*positions)[idx].z = 0.0;
        }

        gl::UnmapBuffer(gl::SHADER_STORAGE_BUFFER);

        gl::UseProgram(program);
        gl::DispatchCompute(10, 1, 1);
        gl::MemoryBarrier(gl::BUFFER_UPDATE_BARRIER_BIT);

        let positions = gl::MapBufferRange(
            gl::SHADER_STORAGE_BUFFER, 0, (NUM_VECS * mem::size_of::<PosVec>()) as isize, gl::MAP_READ_BIT
        ) as *mut [PosVec; NUM_VECS];

        for idx in 0..NUM_VECS {
            println!("x: {}, y: {}, z: {}", (*positions)[idx].x, (*positions)[idx].y, (*positions)[idx].z);
        }

        gl::UnmapBuffer(gl::SHADER_STORAGE_BUFFER);
    }

    // Cleanup
    unsafe {
        gl::DeleteBuffers(1, &pos_ssbo);
        gl::DeleteProgram(program);
    }
}
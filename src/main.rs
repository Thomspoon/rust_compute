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

fn find_sdl_gl_driver() -> Option<u32> {
    for (index, item) in sdl2::render::drivers().enumerate() {
        if item.name == "opengl" {
            return Some(index as u32);
        }
    }
    None
}

// Constants
const NUM_VECS: usize = 10;
const NUM_WORK_GROUPS: usize = 10;
const BIT_MASK: u32 = gl::MAP_WRITE_BIT | gl::MAP_INVALIDATE_BUFFER_BIT;

// Structure definitions
struct PosVec {
    x: f32,
    y: f32,
    z: f32,
}

// Shader sources
static CS_SRC: &'static str =
   "#version 310 es

    // layout (std140, binding = 4) buffer Pos
    // {
    //     vec3 Positions[];
    // };

    // layout(local_size_x=10, local_size_y=1, local_size_z=1) in;

    void main()
    {
        // uint gid = gl_GlobalInvocationID.x;
        // Positions[gid].xyz = vec3(5.0, 5.0, 5.0);
    }
    ";

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    let shader;

    unsafe {
        shader = gl::CreateShader(ty);

        println!("Shader is {:?}", shader);

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
            buf.set_len(len as usize); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);

            panic!("Compile error: {}", str::from_utf8(&buf).ok().expect("ShaderInfoLog not valid utf8"));
        }
    }
    shader
}

fn link_program(cs: GLuint) -> GLuint { 
    let program;
    unsafe {
        program = gl::CreateProgram();
        gl::AttachShader(program, cs);
        gl::LinkProgram(program);

        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);

            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            
            panic!("{}", str::from_utf8(&buf).ok().expect("ProgramInfoLog not valid utf8"));
        }
    } 
    program
}

fn main() {

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(4, 1);

    let window = video_subsystem.window("Window", 800, 600)
        .resizable()
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let ctx = window.gl_create_context().unwrap();
    gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);

    debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
    debug_assert_eq!(gl_attr.context_version(), (4, 1));

    // Create GLSL shaders
    let cs = compile_shader(CS_SRC, gl::COMPUTE_SHADER);
    let mut pos_ssbo: GLuint = 0;

    let program;

    unsafe {
        gl::GenBuffers(1, &mut pos_ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, pos_ssbo);
        gl::BufferData(gl::SHADER_STORAGE_BUFFER, (NUM_VECS * mem::size_of::<PosVec>()) as isize, ptr::null(), gl::STATIC_DRAW);

        let positions = gl::MapBufferRange(
            gl::SHADER_STORAGE_BUFFER, 0, (NUM_VECS * mem::size_of::<PosVec>()) as isize, BIT_MASK
        ) as *mut [PosVec; NUM_VECS];

        for idx in 0..NUM_VECS {
            (*positions)[idx].x = 0.0;
            (*positions)[idx].y = 0.0;
            (*positions)[idx].z = 0.0;
        }

        gl::UnmapBuffer(gl::SHADER_STORAGE_BUFFER);
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 4, pos_ssbo);
        program = link_program(cs);

        gl::DispatchCompute((NUM_VECS / NUM_WORK_GROUPS) as u32, 1, 1);
        gl::MemoryBarrier(gl::SHADER_STORAGE_BARRIER_BIT);

        println!("{:?}", positions);
    }

    // Cleanup
    unsafe {
        gl::DeleteBuffers(1, &pos_ssbo);
        gl::DeleteProgram(program);
    }
}
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;

use glium::{Display, IndexBuffer, VertexBuffer};
use glium::glutin::surface::WindowSurface;
use glium::index::{IndexBufferAny, PrimitiveType};
use glium::vertex::VertexBufferAny;

pub(crate) fn load_shader(path: &str) -> String {
    let mut shader: String = String::new();
    File::open(PathBuf::from(path)).unwrap()
        .read_to_string(&mut shader).unwrap();
    shader
}

pub(crate) const VERT_SHADER_SRC: &str = r"./src/resources/shaders/vert/vert_shader_src.vert";
pub(crate) const VERT_SHADER_BUF: &str = r"./src/resources/shaders/vert/vert_shader_buf.vert";
pub(crate) const FRAG_SHADER_SRC: &str = r"./src/resources/shaders/frag/frag_shader_src.frag";
pub(crate) const FRAG_SHADER_BUF: &str = r"./src/resources/shaders/frag/frag_shader_buf.frag";
pub(crate) const FRAG_SHADER_NRM: &str = r"./src/resources/shaders/frag/frag_shader_nrm.frag";

pub(crate) const COMP_SHADER_DBG: &str = r"./src/resources/shaders/comp/comp_shader_dbg.comp";
pub(crate) const COMP_SHADER_CNV: &str = r"./src/resources/shaders/comp/comp_shader_cnv.comp";



#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub(crate) position: [f32;4],
    pub(crate) normal: [f32;4],
}
implement_vertex!(Vertex, position, normal); //DOWN HERE IS THE STUFF FOR LOADING AND PARSING .obj wavefront FILES.
//                                         I'm wanting to switch to another, probably an own format. more things for the future
pub(crate) fn wavefront(display: &Display<WindowSurface>, prim: PrimitiveType, path: PathBuf)
                        -> (VertexBufferAny, IndexBufferAny)
{
/*
    let mut reader = BufReader::new(File::open(PathBuf::from(r"./src/resources/4/octachoron.sin")).unwrap());
    let mut content = reader.lines().map(|x|x.unwrap()).collect_vec();
    print!("{:?}", content);
 */

    let mut reader = BufReader::new(File::open(path).unwrap()).lines();
    //this is just awful to look at. so many .unwraps() you'd think this is an onion with all those layers
    let dim: usize = reader.next().unwrap().unwrap().parse::<usize>().unwrap();
    let vert_count: usize = reader.next().unwrap().unwrap().parse::<usize>().unwrap();
    let ngon_count: usize = reader.next().unwrap().unwrap().parse::<usize>().unwrap();
    let ngon_size: usize = reader.next().unwrap().unwrap().parse::<usize>().unwrap();

    let mut positions: Vec<_> = Vec::with_capacity(vert_count);
    let mut normals: Vec<_> = Vec::with_capacity(vert_count);
    let mut indices: Vec<(u16,u16)> = Vec::with_capacity(ngon_count*(ngon_size+1));

    reader.for_each(|line|{ //many unreadable string operations
        let mut line = line.unwrap();
        line.push(' ');
        line.push(' ');
        match &line[0..2] {
            "v " => { //if a line in the file begins with "v ". vertex definition here.
                let mut pos: Vec<f32> = line[2..]
                    .split(' ').filter(|c|!c.is_empty())
                    .map(|x|x.parse::<f32>().unwrap()).collect();
                if dim == 3 { pos.push(0.0f32) }
                let pos: [f32;4] = pos.try_into().unwrap();
                positions.push(pos);
            },

            "vn" => { //if a line in the file begins with "vn". vertex normals r here.
                let mut norm: Vec<_> = line[3..]
                    .split(' ').filter(|c|!c.is_empty())
                    .map(|x|x.parse::<f32>().unwrap()).collect();
                if dim == 3 { norm.push(0.0f32) }
                let norm: [f32;4] = norm.try_into().unwrap();
                normals.push(norm);
            },

            "f " => { //if a line in the file begins with "f ". "face" definition's here.
                let arr = line[2..].split(' ').filter(|c|!c.is_empty()).collect::<Vec<&str>>();
                for i in 0..ngon_size {
                    let index_term = arr[i].split('/').collect::<Vec<&str>>();
                    indices.push((index_term[0].parse::<u16>().unwrap(), index_term[2].parse::<u16>().unwrap()));
                };},
            _ => (),
    }});

    //it does what it does. calculates the inverse of the highest value found in the list of vertex positions.
    let mut inv_max = 1.0/positions.iter()
        .map(
            |x| x.iter().fold(0.0f32, |y,x| y+x )
        ).fold(0.0f32,
               |y,x| y+(x-y)*f32::from(x>y)
    );

    inv_max = inv_max*1.1;

    //scale all position vectors so the "longest" one has length 1.
    positions = positions.iter()
        .map(
            |x| x.map(|x|x*inv_max)
        ).collect();


    let mut vertices: Vec<Vertex> = Vec::with_capacity(vert_count);

    //the code for making the index buffer actually work. why it's needed and how it works is explained in the MATURAARBEIT.
    let mut adjustment = 0;
    for mut i in 0..indices.len() {

        if let Some(&vertex) = &vertices.get(indices[i].0 as usize) {

            //assert_eq!(vertex.normal, normals[indices[i].1 as usize]);
            if (vertex.normal != normals[indices[i].1 as usize])
            &  (vertex.position == positions[indices[i].0 as usize])
            {
                let position = positions[indices[i].0 as usize];
                let normal = normals[indices[i].1 as usize];

                i -= adjustment;
                indices[i-1].0 = positions.len() as u16;
                positions.push(position);

                vertices.push(Vertex {
                    position, normal, });
                adjustment += 1;
            }
        } else {

        vertices.push(Vertex {
            position: positions[indices[i].0 as usize],
            normal: normals[indices[i].1 as usize],
        })
        }
    }

    //some more weird adjustments needed for wireframe shading with quads
    if (ngon_size%4              ==    0)
    &  (PrimitiveType::LineStrip == prim)
    {
        for i in 0..indices.len()/4 {
            let adj = 2*((i+1)%2);
            indices.swap(4*i+adj,4*i+adj+1);
        }
    }

    //it works but it's far from readable. simply adds a "restart index" every few steps.
    // "clone().iter().cloned()" and "::<u16,u16,Vec<u16>,Vec<u16>>)" and any amount of wildcards in the type definitions is instantly a mess.
    // functional programing stuff, I guess...
    (0..indices.len()).for_each(|i| if i.rem_euclid(ngon_size) == 0 { indices.insert(i + i / ngon_size, (u16::MAX, u16::MAX)) });
    let (indices,_): (Vec<_>,_) = indices.clone().iter().cloned().unzip::<u16,u16,Vec<u16>,Vec<u16>>();

    //dbg!(&indices);

    match prim {
        PrimitiveType::TriangleStrip => (VertexBuffer::new(display, &vertices).unwrap().into(),
                                         IndexBuffer::new(display, PrimitiveType::TriangleStrip, &indices).unwrap().into()),
        PrimitiveType::LineStrip =>     (VertexBuffer::new(display, &vertices).unwrap().into(),
                                         IndexBuffer::new(display, PrimitiveType::LineLoop, &indices).unwrap().into()),
        _ => panic!()
    }

}
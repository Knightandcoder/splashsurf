use anyhow::{anyhow, Context};
use splashsurf_lib::mesh::{AttributeData, MeshAttribute, MeshWithData, TriMesh3d};
use splashsurf_lib::nalgebra::Vector3;
use splashsurf_lib::vtkio;
use splashsurf_lib::vtkio::model::{
    Attribute, Attributes, CellType, Cells, PolyDataPiece, UnstructuredGridPiece, VertexNumbers,
};
use splashsurf_lib::{IteratorExt, Real};
use std::borrow::Cow;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use vtkio::model::{ByteOrder, DataSet, Version, Vtk};
use vtkio::IOBuffer;

pub struct VtkFile {
    pieces: Vec<DataPiece>,
}

pub enum DataPiece {
    UnstructuredGrid(UnstructuredGridPiece),
    PolyData(PolyDataPiece),
}

impl VtkFile {
    /// Loads all pieces of the given VTK struct
    pub fn from_vtk(vtk_file: Vtk) -> Result<Self, anyhow::Error> {
        let loaded_pieces =
            load_pieces(vtk_file).context(anyhow!("Failed to load all pieces from VTK file"))?;

        Ok(Self {
            pieces: loaded_pieces,
        })
    }

    /// Loads a VTK file from the given path and loads all its data pieces
    pub fn load_file<P: AsRef<Path>>(file_path: P) -> Result<Self, anyhow::Error> {
        let file_path = file_path.as_ref();
        let vtk_file = read_vtk(file_path)
            .with_context(|| anyhow!("Failed to load VTK file \"{}\"", file_path.display()))?;

        Self::from_vtk(vtk_file)
    }

    /// Returns all pieces that could be loaded from the VTK file
    pub fn into_pieces(self) -> Vec<DataPiece> {
        self.pieces
    }
}

impl DataPiece {
    /// Returns a slice of all point attributes of this data piece
    fn point_attributes(&self) -> &[Attribute] {
        match self {
            DataPiece::UnstructuredGrid(p) => &p.data.point,
            DataPiece::PolyData(p) => &p.data.point,
        }
    }

    /// Returns the names of all supported point attributes of the given piece
    pub fn point_attribute_names(&self) -> Vec<String> {
        attribute_names(self.point_attributes())
    }

    /// Tries to load a set of particles form this piece
    pub fn load_as_particles<R: Real>(&self) -> Result<Vec<Vector3<R>>, anyhow::Error> {
        let points = match self {
            DataPiece::UnstructuredGrid(p) => &p.points,
            DataPiece::PolyData(p) => &p.points,
        };

        match points {
            IOBuffer::F64(coords) => particles_from_coords(&coords),
            IOBuffer::F32(coords) => particles_from_coords(&coords),
            _ => Err(anyhow!(
                "Point coordinate IOBuffer does not contain f32 or f64 values"
            )),
        }
    }

    /// Tries to load a surface mesh from this piece
    pub fn load_as_surface_mesh<R: Real>(
        &self,
    ) -> Result<MeshWithData<R, TriMesh3d<R>>, anyhow::Error> {
        match self {
            DataPiece::UnstructuredGrid(p) => surface_mesh_from_unstructured_grid(p),
            //DataPiece::PolyData(p) => unimplemented!(),
            _ => Err(anyhow!("Unsupported piece type for loading surface mesh")),
        }
    }

    /// Tries to load attributes with the given names from the data piece, returns an error if the attribute does not exist
    pub fn load_point_attributes<R: Real>(
        &self,
        names: &[String],
    ) -> Result<Vec<MeshAttribute<R>>, anyhow::Error> {
        let mut mesh_attributes = Vec::new();

        // Converts an IOBuffer to the corresponding supported AttributeData
        let io_buffer_to_attribute_data = |num_comp: usize,
                                           io_buffer: &vtkio::model::IOBuffer|
         -> Result<AttributeData<R>, anyhow::Error> {
            match num_comp {
                1 => {
                    if let IOBuffer::U32(vec) = &io_buffer {
                        Ok(AttributeData::ScalarReal(
                            vec.iter()
                                .copied()
                                .map(|val| R::from_u32(val).unwrap())
                                .collect(),
                        ))
                    } else {
                        Err(anyhow!("Unsupported IOBuffer scalar type"))
                    }
                }
                3 => match &io_buffer {
                    IOBuffer::F32(coords) => {
                        particles_from_coords(coords).map(|p| AttributeData::Vector3Real(p))
                    }
                    IOBuffer::F64(coords) => {
                        particles_from_coords(coords).map(|p| AttributeData::Vector3Real(p))
                    }
                    _ => Err(anyhow!("Unsupported IOBuffer vector type")),
                },
                _ => Err(anyhow!(
                    "Unsupported number of components ({}) in VTK DataArray",
                    num_comp
                )),
            }
        };

        'fields: for field_name in names {
            for attribute in self.point_attributes() {
                match attribute {
                    Attribute::DataArray(data_array) if data_array.name == *field_name => {
                        let attribute_data =
                            io_buffer_to_attribute_data(data_array.num_comp(), &data_array.data)
                                .with_context(|| anyhow!("Attribute \"{}\"", field_name))?;
                        let mesh_attribute = MeshAttribute::new(field_name, attribute_data);
                        mesh_attributes.push(mesh_attribute);
                        continue 'fields;
                    }
                    Attribute::Field { data_array, .. } => {
                        for field_array in data_array {
                            if field_array.name == *field_name {
                                let attribute_data = io_buffer_to_attribute_data(
                                    field_array.num_comp(),
                                    &field_array.data,
                                )
                                .with_context(|| anyhow!("Attribute \"{}\"", field_name))?;
                                let mesh_attribute = MeshAttribute::new(field_name, attribute_data);
                                mesh_attributes.push(mesh_attribute);
                                continue 'fields;
                            }
                        }
                    }
                    _ => {}
                }
            }

            return Err(anyhow!("Attribute {} not found in VTK file", field_name));
        }

        Ok(mesh_attributes)
    }
}

/// Tries to read a set of particles from the VTK file at the given path
pub fn particles_from_vtk<R: Real, P: AsRef<Path>>(
    file_path: P,
) -> Result<Vec<Vector3<R>>, anyhow::Error> {
    let file_path = file_path.as_ref();
    VtkFile::load_file(file_path)?
        .into_pieces()
        .first()
        .ok_or_else(|| {
            anyhow!(
                "No supported pieces in VTK file \"{}\"",
                file_path.display()
            )
        })?
        .load_as_particles()
}

/// Tries to write a set of particles to the VTK file at the given path
pub fn particles_to_vtk<R: Real, P: AsRef<Path>>(
    particles: &[Vector3<R>],
    vtk_file: P,
) -> Result<(), anyhow::Error> {
    write_vtk(
        UnstructuredGridPiece::from(Particles(particles)),
        vtk_file,
        "particles",
    )
}

/// Tries to read a surface mesh from the VTK file at the given path
pub fn surface_mesh_from_vtk<R: Real, P: AsRef<Path>>(
    file_path: P,
) -> Result<MeshWithData<R, TriMesh3d<R>>, anyhow::Error> {
    let file_path = file_path.as_ref();
    VtkFile::load_file(file_path)?
        .into_pieces()
        .first()
        .ok_or_else(|| {
            anyhow!(
                "No supported pieces in VTK file \"{}\"",
                file_path.display()
            )
        })?
        .load_as_surface_mesh()
}

/// Tries to write `data` that is convertible to a VTK `DataSet` into a big endian VTK file
pub fn write_vtk<P: AsRef<Path>>(
    data: impl Into<DataSet>,
    filename: P,
    title: &str,
) -> Result<(), anyhow::Error> {
    let vtk_file = Vtk {
        version: Version::new((4, 1)),
        title: title.to_string(),
        file_path: None,
        byte_order: ByteOrder::BigEndian,
        data: data.into(),
    };

    let filename = filename.as_ref();
    if let Some(dir) = filename.parent() {
        create_dir_all(dir).context("Failed to create parent directory of output file")?;
    }
    vtk_file
        .export_be(filename)
        .context("Error while writing VTK output to file")
}

/// Tries to read the given VTK file
pub fn read_vtk<P: AsRef<Path>>(filename: P) -> Result<Vtk, anyhow::Error> {
    let filename = filename.as_ref();
    let mut vtk_file = Vtk::import(filename)?;
    vtk_file.load_all_pieces()?;
    Ok(vtk_file)
}

/// Loads all supported pieces of the given VTK file
fn load_pieces(vtk_file: Vtk) -> Result<Vec<DataPiece>, anyhow::Error> {
    let file_path = vtk_file.file_path.as_ref().map(PathBuf::as_path);

    let loaded_pieces = match vtk_file.data {
        DataSet::UnstructuredGrid { pieces, .. } => pieces
            .into_iter()
            .map(|p| p.into_loaded_piece_data(file_path))
            .map(|p| p.map(|p| DataPiece::UnstructuredGrid(p)))
            .collect::<Result<Vec<_>, _>>()?,
        DataSet::PolyData { pieces, .. } => pieces
            .into_iter()
            .map(|p| p.into_loaded_piece_data(file_path))
            .map(|p| p.map(|p| DataPiece::PolyData(p)))
            .collect::<Result<Vec<_>, _>>()?,
        _ => Err(anyhow!(
            "VTK file does not contain supported data set pieces"
        ))?,
    };

    Ok(loaded_pieces)
}

/// Collects the names of all supported attributes in the given slice
fn attribute_names(attributes: &[Attribute]) -> Vec<String> {
    let mut attribute_names = Vec::new();

    for attribute in attributes {
        match attribute {
            // A `DataArray` contains only a single attribute (with a name and values inside of an `IOBuffer`)
            Attribute::DataArray(data) => attribute_names.push(data.name.clone()),
            // A `Field` is an array of array, we only check its children and ignore its own name
            Attribute::Field { data_array, .. } => {
                for data in data_array {
                    attribute_names.push(data.name.clone());
                }
            }
        }
    }

    attribute_names
}

/// Tries to construct a surface mesh from the given grid piece
fn surface_mesh_from_unstructured_grid<R: Real>(
    piece: &UnstructuredGridPiece,
) -> Result<MeshWithData<R, TriMesh3d<R>>, anyhow::Error> {
    let vertices = match &piece.points {
        IOBuffer::F64(coords) => particles_from_coords(coords),
        IOBuffer::F32(coords) => particles_from_coords(coords),
        _ => Err(anyhow!(
            "Point coordinate IOBuffer does not contain f32 or f64 values"
        )),
    }?;

    let triangles = {
        let (num_cells, cell_verts) = match &piece.cells.cell_verts {
            VertexNumbers::Legacy {
                num_cells,
                vertices,
            } => (*num_cells, Cow::Borrowed(vertices)),
            xml @ VertexNumbers::XML { .. } => {
                let (num_cells, cell_verts) = xml.clone().into_legacy();
                (num_cells, Cow::Owned(cell_verts))
            }
        };

        if cell_verts.len() % 4 != 0 {
            return Err(anyhow!("Length of cell vertex array is invalid. Expected 4 values per cell (3 for each triangle vertex index + 1 for vertex count). There are {} values for {} cells.", cell_verts.len(), num_cells));
        }

        let cells = cell_verts
            .chunks_exact(4)
            .enumerate()
            .map(|(cell_idx, cell)| {
                let is_triangle = cell[0] == 0;
                is_triangle
                    .then(|| [cell[1] as usize, cell[2] as usize, cell[3] as usize])
                    .ok_or_else(|| anyhow!("Expected only triangle cells. Invalid number of vertex indices ({}) of cell {}", cell[0], cell_idx))
            })
            .try_collect_with_capacity(num_cells as usize)?;
        cells
    };

    Ok(MeshWithData::new(TriMesh3d {
        vertices,
        triangles,
    }))
}

/// Tries to convert a vector of consecutive coordinate triplets into a vector of `Vector3`, also converts between floating point types
fn particles_from_coords<RealOut: Real, RealIn: Real>(
    coords: &Vec<RealIn>,
) -> Result<Vec<Vector3<RealOut>>, anyhow::Error> {
    if coords.len() % 3 != 0 {
        return Err(anyhow!(
            "Particle point buffer length is not divisible by 3"
        ));
    }

    let num_points = coords.len() / 3;
    let positions = coords
        .chunks_exact(3)
        .map(|triplet| {
            Some(Vector3::new(
                triplet[0].try_convert()?,
                triplet[1].try_convert()?,
                triplet[2].try_convert()?,
            ))
        })
        .map(|vec| {
            vec.ok_or_else(|| {
                anyhow!("Failed to convert coordinate from input to output float type, value out of range?")
            })
        })
        .try_collect_with_capacity(num_points)?;

    Ok(positions)
}

/// Wrapper for a slice of particle positions for converting it into a VTK `UnstructuredGridPiece`
struct Particles<'a, R: Real>(&'a [Vector3<R>]);

impl<'a, R> From<Particles<'a, R>> for UnstructuredGridPiece
where
    R: Real,
{
    fn from(particles: Particles<'a, R>) -> Self {
        let particles = particles.0;

        let points = {
            let mut points: Vec<R> = Vec::with_capacity(particles.len() * 3);
            for p in particles.iter() {
                points.extend(p.as_slice());
            }
            points
        };

        // Each particle has a cell of type `Vertex`
        let cell_types = vec![CellType::Vertex; particles.len()];

        let vertices = {
            let mut vertices = Vec::with_capacity(particles.len() * (1 + 1));
            for i in 0..particles.len() {
                // Number of vertices of the cell
                vertices.push(1);
                // Vertex index
                vertices.push(i as u32);
            }
            vertices
        };

        UnstructuredGridPiece {
            points: points.into(),
            cells: Cells {
                cell_verts: VertexNumbers::Legacy {
                    num_cells: cell_types.len() as u32,
                    vertices,
                },
                types: cell_types,
            },
            data: Attributes::new(),
        }
    }
}

//!
//! Library for surface reconstruction using marching cubes for SPH particle data. Entry point is the [reconstruct_surface] function.
//!

/// Re-export the version of coarse_prof used by this crate, if profiling is enabled
#[cfg(feature = "profiling")]
pub use coarse_prof;
/// Re-export the version of nalgebra used by this crate
pub use nalgebra;
/// Re-export the version of vtkio used by this crate, if vtk support is enabled
#[cfg(feature = "vtk_extras")]
pub use vtkio;

#[cfg(feature = "profiling")]
/// Invokes coarse_prof::profile! with the given expression
macro_rules! profile {
    ($body:expr) => {
        coarse_prof::profile!($body);
    };
}

#[cfg(not(feature = "profiling"))]
/// No-op macro if profiling is disabled
macro_rules! profile {
    ($body:expr) => {
        $body
    };
}

mod aabb;
/// Computation of sparse density maps (evaluation of particle densities and mapping onto sparse grids)
pub mod density_map;
/// Generic octree implementation for basic algorithms
pub mod generic_tree;
/// SPH kernel function implementations
pub mod kernel;
/// Triangulation of density maps using marching cubes
pub mod marching_cubes;
pub mod marching_cubes_lut;
/// Basic mesh types used by the library and implementation of VTK export
pub mod mesh;
/// Simple neighborhood search based on spatial hashing
pub mod neighborhood_search;
mod numeric_types;
/// Octree implementation to spatially partition particle sets
pub mod octree;
/// Functions calling the individual steps of the reconstruction pipeline
mod reconstruction;
/// Helper types for cartesian coordinate system topology
pub mod topology;
/// Types related to the virtual background grid used for marching cubes
pub mod uniform_grid;
/// Internal helper functions and types
mod utils;
/// Workspace for reusing allocated memory between multiple reconstructions
pub(crate) mod workspace;

use log::info;
use nalgebra::Vector3;
use thiserror::Error as ThisError;

use crate::mesh::TriMesh3d;
use crate::octree::Octree;
use crate::reconstruction::{
    reconstruct_single_surface_append, SurfaceReconstructionOctreeVisitor,
};
use crate::workspace::ReconstructionWorkspace;
pub use aabb::{AxisAlignedBoundingBox, AxisAlignedBoundingBox2d, AxisAlignedBoundingBox3d};
pub use density_map::DensityMap;
pub use numeric_types::{Index, Real, ThreadSafe};
pub use octree::SubdivisionCriterion;
pub use uniform_grid::{GridConstructionError, UniformGrid};

// TODO: Add documentation of feature flags
// TODO: Feature flag for multi threading
// TODO: Feature flag to disable (debug level) logging?

// TODO: Remove anyhow/thiserror from lib?
// TODO: Write more unit tests (e.g. AABB, UniformGrid, neighborhood search)
// TODO: Write some integration tests
// TODO: Test kernels with property based testing?
// TODO: Add free particles back again after triangulation as sphere meshes if they were removed
// TODO: Detect free particles by just comparing with the SPH density of a free particle? (no need for extra neighborhood search?)
// TODO: More and better error messages with distinct types
// TODO: Make flat indices strongly typed
// TODO: Function that detects smallest usable index type

pub(crate) type HashState = fxhash::FxBuildHasher;
pub(crate) type MapType<K, V> = std::collections::HashMap<K, V, HashState>;
pub(crate) fn new_map<K, V>() -> MapType<K, V> {
    MapType::with_hasher(HashState::default())
}

/*
// Switch to BTreeMap in debug mode for easier debugging due to deterministic iteration order
#[cfg(debug_assertions)]
pub(crate) type MapType<K, V> = std::collections::BTreeMap<K, V>;
#[cfg(not(debug_assertions))]
pub(crate) type MapType<K, V> = std::collections::HashMap<K, V, HashState>;

// Function for consistent construction of the used map type (depending on debug/release build)
#[cfg(debug_assertions)]
pub(crate) fn new_map<K: std::cmp::Ord, V>() -> MapType<K, V> {
    MapType::new()
}
#[cfg(not(debug_assertions))]
pub(crate) fn new_map<K, V>() -> MapType<K, V> {
    MapType::with_hasher(HashState::default())
}
*/

pub(crate) type ParallelMapType<K, V> = dashmap::DashMap<K, V, HashState>;

/// Macro version of Option::map that allows using e.g. using the ?-operator in the map expression
macro_rules! map_option {
    ($some_optional:expr, $value_identifier:ident => $value_transformation:expr) => {
        match $some_optional {
            Some($value_identifier) => Some($value_transformation),
            None => None,
        }
    };
}

/// Parameters for the spatial decomposition
#[derive(Clone, Debug)]
pub struct SpatialDecompositionParameters<R: Real> {
    /// Criterion used for subdivision of the octree cells
    pub subdivision_criterion: SubdivisionCriterion,
    /// Safety factor applied to the kernel radius when it's used as a margin to collect ghost particles in the leaf nodes
    pub ghost_particle_safety_factor: Option<R>,
    /// Whether to enable stitching of all disjoint subdomain meshes to a global manifold mesh
    pub enable_stitching: bool,
}

impl<R: Real> SpatialDecompositionParameters<R> {
    /// Tries to convert the parameters from one [Real] type to another [Real] type, returns None if conversion fails
    pub fn try_convert<T: Real>(&self) -> Option<SpatialDecompositionParameters<T>> {
        Some(SpatialDecompositionParameters {
            subdivision_criterion: self.subdivision_criterion.clone(),
            ghost_particle_safety_factor: map_option!(
                &self.ghost_particle_safety_factor,
                r => r.try_convert()?
            ),
            enable_stitching: self.enable_stitching,
        })
    }
}

/// Parameters for the surface reconstruction
#[derive(Clone, Debug)]
pub struct Parameters<R: Real> {
    /// Radius per particle (used to calculate the particle volume)
    pub particle_radius: R,
    /// Rest density of the fluid
    pub rest_density: R,
    /// Compact support radius of the kernel, i.e. distance from the particle where kernel reaches zero (in distance units, not relative to particle radius)
    pub kernel_radius: R,
    /// Particles without neighbors within the splash detection radius are considered "splash" or "free particles".
    /// They are filtered out and processed separately. Currently they are only skipped during the surface reconstruction.
    pub splash_detection_radius: Option<R>,
    /// Edge length of the marching cubes implicit background grid (in distance units, not relative to particle radius)
    pub cube_size: R,
    /// Density threshold value to distinguish between the inside (above threshold) and outside (below threshold) of the fluid
    pub iso_surface_threshold: R,
    /// Manually restrict the domain to the surface reconstruction.
    /// If not provided, the smallest AABB enclosing all particles is computed instead.
    pub domain_aabb: Option<AxisAlignedBoundingBox3d<R>>,
    /// Whether to allow multi threading within the surface reconstruction procedure
    pub enable_multi_threading: bool,
    /// Parameters for the spatial decomposition (octree subdivision) of the particles.
    /// If not provided, no octree is generated and a global approach is used instead.
    pub spatial_decomposition: Option<SpatialDecompositionParameters<R>>,
}

impl<R: Real> Parameters<R> {
    /// Tries to convert the parameters from one [Real] type to another [Real] type, returns None if conversion fails
    pub fn try_convert<T: Real>(&self) -> Option<Parameters<T>> {
        Some(Parameters {
            particle_radius: self.particle_radius.try_convert()?,
            rest_density: self.rest_density.try_convert()?,
            kernel_radius: self.kernel_radius.try_convert()?,
            splash_detection_radius: map_option!(
                &self.splash_detection_radius,
                r => r.try_convert()?
            ),
            cube_size: self.cube_size.try_convert()?,
            iso_surface_threshold: self.iso_surface_threshold.try_convert()?,
            domain_aabb: map_option!(&self.domain_aabb, aabb => aabb.try_convert()?),
            enable_multi_threading: self.enable_multi_threading,
            spatial_decomposition: map_option!(&self.spatial_decomposition, sd => sd.try_convert()?),
        })
    }
}

/// Result data returned when the surface reconstruction was successful
#[derive(Clone, Debug)]
pub struct SurfaceReconstruction<I: Index, R: Real> {
    /// Background grid that was used as a basis for generating the density map for marching cubes
    grid: UniformGrid<I, R>,
    /// Octree built for domain decomposition
    octree: Option<Octree<I, R>>,
    /// Point-based density map generated from the particles that was used as input to marching cubes
    density_map: Option<DensityMap<I, R>>,
    /// Surface mesh that is the result of the surface reconstruction
    mesh: TriMesh3d<R>,
    /// Workspace with allocated memory for subsequent surface reconstructions
    workspace: ReconstructionWorkspace<I, R>,
}

impl<I: Index, R: Real> Default for SurfaceReconstruction<I, R> {
    /// Returns an empty [SurfaceReconstruction] to pass into the inplace surface reconstruction
    fn default() -> Self {
        Self {
            grid: UniformGrid::new_zero(),
            octree: None,
            density_map: None,
            mesh: TriMesh3d::default(),
            workspace: ReconstructionWorkspace::default(),
        }
    }
}

impl<I: Index, R: Real> SurfaceReconstruction<I, R> {
    /// Returns a reference to the actual triangulated surface mesh that is the result of the reconstruction
    pub fn mesh(&self) -> &TriMesh3d<R> {
        &self.mesh
    }

    /// Returns a reference to the octree generated for spatial decomposition of the input particles
    pub fn octree(&self) -> Option<&Octree<I, R>> {
        self.octree.as_ref()
    }

    /// Returns a reference to the sparse density map (discretized on the vertices of the background grid) that is used as input for marching cubes
    pub fn density_map(&self) -> Option<&DensityMap<I, R>> {
        self.density_map.as_ref()
    }

    /// Returns a reference to the virtual background grid that was used as a basis for discretization of the density map for marching cubes, can be used to convert the density map to a hex mesh (using [sparse_density_map_to_hex_mesh](density_map::sparse_density_map_to_hex_mesh))
    pub fn grid(&self) -> &UniformGrid<I, R> {
        &self.grid
    }
}

impl<I: Index, R: Real> From<SurfaceReconstruction<I, R>> for TriMesh3d<R> {
    /// Extracts the reconstructed mesh
    fn from(result: SurfaceReconstruction<I, R>) -> Self {
        result.mesh
    }
}

/// Error type returned when the surface reconstruction fails
#[non_exhaustive]
#[derive(Debug, ThisError)]
pub enum ReconstructionError<I: Index, R: Real> {
    /// Errors that occur during the implicit construction of the virtual background grid used for the density map and marching cubes
    #[error("grid construction: {0}")]
    GridConstructionError(GridConstructionError<I, R>),
    /// Any error that is not represented by some other explicit variant
    #[error("unknown error")]
    Unknown(anyhow::Error),
}

impl<I: Index, R: Real> From<GridConstructionError<I, R>> for ReconstructionError<I, R> {
    /// Allows automatic conversion of a [GridConstructionError] to a [ReconstructionError]
    fn from(error: GridConstructionError<I, R>) -> Self {
        ReconstructionError::GridConstructionError(error)
    }
}

impl<I: Index, R: Real> From<anyhow::Error> for ReconstructionError<I, R> {
    /// Allows automatic conversion of an anyhow::Error to a [ReconstructionError]
    fn from(error: anyhow::Error) -> Self {
        ReconstructionError::Unknown(error)
    }
}

/// Initializes the global thread pool used by this library with the given parameters.
///
/// Initialization of the global thread pool happens exactly once.
/// Therefore, if you call `initialize_thread_pool` a second time, it will return an error.
/// An `Ok` result indicates that this is the first initialization of the thread pool.
pub fn initialize_thread_pool(num_threads: usize) -> Result<(), anyhow::Error> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()?;
    Ok(())
}

/// Performs a marching cubes surface construction of the fluid represented by the given particle positions
#[inline(never)]
pub fn reconstruct_surface<I: Index, R: Real>(
    particle_positions: &[Vector3<R>],
    parameters: &Parameters<R>,
) -> Result<SurfaceReconstruction<I, R>, ReconstructionError<I, R>> {
    profile!("reconstruct_surface");
    let mut surface = SurfaceReconstruction::default();
    reconstruct_surface_inplace(particle_positions, parameters, &mut surface)?;
    Ok(surface)
}

/// Performs a marching cubes surface construction of the fluid represented by the given particle positions, inplace
pub fn reconstruct_surface_inplace<'a, I: Index, R: Real>(
    particle_positions: &[Vector3<R>],
    parameters: &Parameters<R>,
    output_surface: &'a mut SurfaceReconstruction<I, R>,
) -> Result<(), ReconstructionError<I, R>> {
    // Clear the existing mesh
    output_surface.mesh.clear();

    // Initialize grid for the reconstruction
    output_surface.grid = grid_for_reconstruction(
        particle_positions,
        parameters.particle_radius,
        parameters.kernel_radius,
        parameters.cube_size,
        parameters.domain_aabb.as_ref(),
        parameters.enable_multi_threading,
    )?;
    let grid = &output_surface.grid;
    grid.log_grid_info();

    if parameters.spatial_decomposition.is_some() {
        SurfaceReconstructionOctreeVisitor::new(particle_positions, parameters, output_surface)
            .unwrap()
            .run(particle_positions, output_surface);
    } else {
        profile!("reconstruct_surface_inplace");

        let mut workspace = output_surface
            .workspace
            .get_local_with_capacity(particle_positions.len())
            .borrow_mut();

        // Clear the current mesh, as reconstruction will be appended to output
        output_surface.mesh.clear();
        // Perform global reconstruction without octree
        reconstruct_single_surface_append(
            &mut *workspace,
            grid,
            None,
            particle_positions,
            parameters,
            &mut output_surface.mesh,
        );

        /*
        let particle_indices = splash_detection_radius.map(|splash_detection_radius| {
            let neighborhood_list = neighborhood_search::search::<I, R>(
                &grid.aabb(),
                particle_positions,
                splash_detection_radius,
                enable_multi_threading,
            );

            let mut active_particles = Vec::new();
            for (particle_i, neighbors) in neighborhood_list.iter().enumerate() {
                if !neighbors.is_empty() {
                    active_particles.push(particle_i);
                }
            }

            active_particles
        });
        */

        // TODO: Set this correctly
        output_surface.density_map = None;
    }

    Ok(())
}

/// Constructs the background grid for marching cubes based on the parameters supplied to the surface reconstruction
pub fn grid_for_reconstruction<I: Index, R: Real>(
    particle_positions: &[Vector3<R>],
    particle_radius: R,
    kernel_radius: R,
    cube_size: R,
    domain_aabb: Option<&AxisAlignedBoundingBox3d<R>>,
    enable_multi_threading: bool,
) -> Result<UniformGrid<I, R>, ReconstructionError<I, R>> {
    let domain_aabb = if let Some(domain_aabb) = domain_aabb {
        domain_aabb.clone()
    } else {
        profile!("compute minimum enclosing aabb");

        let mut domain_aabb = {
            let mut aabb = if enable_multi_threading {
                AxisAlignedBoundingBox3d::from_points_par(particle_positions)
            } else {
                AxisAlignedBoundingBox3d::from_points(particle_positions)
            };
            aabb.grow_uniformly(particle_radius);
            aabb
        };

        info!(
            "Minimal enclosing bounding box of particles was computed as: {:?}",
            domain_aabb
        );

        // Ensure that we have enough margin around the particles such that the every particle's kernel support is completely in the domain
        let kernel_margin =
            density_map::compute_kernel_evaluation_radius::<I, R>(kernel_radius, cube_size)
                .kernel_evaluation_radius;
        domain_aabb.grow_uniformly(kernel_margin);

        domain_aabb
    };

    Ok(UniformGrid::from_aabb(&domain_aabb, cube_size)?)
}

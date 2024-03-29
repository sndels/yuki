import bpy
from bpy_extras.io_utils import ExportHelper
import logging
from mathutils import Vector
import math
import os
import traceback
import struct
import shutil
from typing import TextIO


def auto_register(register: bool):
    yield PBRT_OT_export

    if register:
        bpy.types.TOPBAR_MT_file_export.append(menu_fn)
    else:
        bpy.types.TOPBAR_MT_file_export.remove(menu_fn)


def menu_fn(self, context):
    self.layout.operator(PBRT_OT_export.bl_idname)


# Used to name plys
MESH_COUNT = 0
EXPORT_DIR = ""
EXPORT_WARNINGS = False
EXPORTED_PLY_MESHES = set()
SEEN_TEXTURES = set()


class PBRT_OT_export(bpy.types.Operator, ExportHelper):
    """Export scene as pbrt-v3"""

    bl_idname = "pbrt_blender.export"
    bl_label = "pbrt-v3 (.pbrt)"
    bl_options = {
        "REGISTER",
        "UNDO",
    }  # Enable undo in case we touch the scene by accident

    filename_ext = ".pbrt"

    def execute(self, context):
        global MESH_COUNT
        MESH_COUNT = 0
        depsgraph = context.evaluated_depsgraph_get()
        if export_scene(depsgraph, context.scene, self.filepath):
            if EXPORT_WARNINGS:
                self.report({"WARNING"}, "Scene exported. See console for warnings.")
            else:
                self.report({"INFO"}, "Scene exported")
            return {"FINISHED"}
        else:
            self.report({"ERROR"}, "Scene export failed. See console for errors.")
            return {"CANCELLED"}


logger = logging.getLogger("pbrt-blender")
logger.setLevel(logging.INFO)


def export_scene(depsgraph, scene, filename) -> bool:
    global EXPORT_WARNINGS
    EXPORT_WARNINGS = False

    global EXPORT_DIR
    EXPORT_DIR = os.path.dirname(filename)
    os.makedirs(os.path.join(EXPORT_DIR, "plys"), exist_ok=True)

    global EXPORTED_PLY_MESHES
    EXPORTED_PLY_MESHES = set()

    global SEEN_TEXTURES
    SEEN_TEXTURES = set()

    with open(filename, "w") as f:
        if scene.camera:
            cam_obj = scene.camera
        else:
            cam_obj = None
            for obj in scene.objects:
                if obj.type == "CAMERA":
                    cam_obj = obj
                    break

        assert cam_obj is not None

        cam_trfn = cam_obj.matrix_world
        cam_pos = cam_trfn @ Vector((0.0, 0.0, 0.0))
        cam_target = cam_trfn @ Vector((0.0, 0.0, -1.0))
        # Blender's camera rests facing -Z with +Y up
        cam_up = cam_trfn.inverted().transposed() @ Vector((0.0, 1.0, 0.0))

        f.write(f"LookAt {fstr3(cam_pos[0], cam_pos[2], cam_pos[1])} # eye\n")
        f.write(
            f"       {fstr3(cam_target[0], cam_target[2], cam_target[1])} # target\n"
        )
        f.write(f"       {fstr3(cam_up[0], cam_up[2], cam_up[1])} # up\n")

        f.write(f"Camera")
        cam = cam_obj.data

        if cam.type != "PERSP":
            _error("Only 'perspective' cameras are supported. Active camera is not.")
            return False

        film_w = scene.render.resolution_x * scene.render.pixel_aspect_x
        film_h = scene.render.resolution_y * scene.render.pixel_aspect_y
        if film_h < film_w:
            if cam.sensor_fit == "VERTICAL":
                fov = math.degrees(cam.angle)
            else:
                fov = math.degrees(
                    2
                    * math.atan(math.tan(cam.angle / 2) * float(film_h) / float(film_w))
                )
        else:
            if cam.sensor_fit == "HORIZONTAL":
                fov = math.degrees(cam.angle)
            else:
                fov = math.degrees(
                    2
                    * math.atan(math.tan(cam.angle / 2) * float(film_w) / float(film_h))
                )
        f.write(f' "perspective" "float fov" {fstr(fov)}\n')
        f.write("\n")

        f.write(f'Sampler "halton" "integer pixelsamples" 128\n')
        f.write(f'Integrator "path"\n')

        scene_name = os.path.basename(bpy.data.filepath).split(".")[0]
        f.write(f'Film "image" "string filename" "{scene_name}.png"\n')
        f.write(
            f'     "integer xresolution" [{scene.render.resolution_x}] "integer yresolution" [{scene.render.resolution_y}]\n'
        )
        f.write("\n")

        f.write("WorldBegin\n")
        f.write("\n")

        if "Background" in scene.world.node_tree.nodes:
            bg_node = scene.world.node_tree.nodes["Background"]
            if (
                len(bg_node.outputs["Background"].links) == 1
                and bg_node.outputs["Background"].links[0].to_socket.node.name
                == "World Output"
            ):
                bg_color = bg_node.inputs["Color"].default_value
                strength = bg_node.inputs["Strength"].default_value
                # TODO: Check for links
                f.write(
                    f'LightSource "infinite" "rgb L" [ {fstr3(bg_color[0] * strength, bg_color[1] * strength, bg_color[2] * strength)} ]\n'
                )
                f.write("\n")
        else:
            _warn(
                "Didn't find 'Background' connected to 'World Output' in scene bg material. Bg will be black."
            )

        try:
            _export_collection(depsgraph, scene.collection, f)
        except:
            traceback.print_exc()
            _error("Export failed")
            return False

        _info("Export done")

        f.write("WorldEnd\n")

        return True


def _error(msg: str):
    logger.error(msg)


def _warn(msg: str):
    global EXPORT_WARNINGS
    EXPORT_WARNINGS = True

    logger.warning(msg)


def _info(msg: str):
    logger.info(msg)


def _export_collection(depsgraph, collection, f: TextIO):
    for obj in collection.objects:
        if obj.parent is None:
            _export_obj(depsgraph, obj, f)

    for collection in collection.children:
        _export_collection(depsgraph, collection, f)


def _export_obj(depsgraph, obj, f: TextIO):
    global MESH_COUNT
    global EXPORT_DIR
    global EXPORTED_PLY_MESHES

    trfn = obj.matrix_world
    if obj.type == "LIGHT":
        light = obj.data
        if light.type == "POINT":
            pos = trfn @ Vector((0.0, 0.0, 0.0))
            L = (light.energy * light.color) / (3 * math.pi)

            f.write(f"# {obj.name_full}\n")
            f.write(
                f'LightSource "point" "point from" [ {fstr3(pos[0], pos[2], pos[1])} ] "rgb I" [ {fstr3(L[0], L[1], L[2])} ]\n'
            )
            f.write("\n")
        elif light.type == "SUN":
            from_p = trfn @ Vector((0.0, 0.0, 0.0))
            to_p = trfn @ Vector((0.0, 0.0, -1.0))
            L = (light.energy * light.color) / (3 * math.pi)

            f.write(f"# {obj.name_full}\n")
            f.write(
                f'LightSource "distant" "point from" [ {fstr3(from_p[0], from_p[2], from_p[1])} ] "point to" [ {fstr3(to_p[0], to_p[2], to_p[1])} ] "rgb L" [ {fstr3(L[0], L[1], L[2])} ]\n'
            )
            f.write("\n")
        else:
            logger.info(
                f"{obj.name_full}: Skipping unimplemented light type '{light.type}'"
            )
    elif obj.type == "MESH":
        _info(f"Exporting mesh for {obj.name}")

        pos, qrot, scale = trfn.decompose()
        rot_axis, rot_angle = qrot.to_axis_angle()

        # Get mesh with modifiers applied
        evaluated_obj = obj.evaluated_get(depsgraph)
        mesh = evaluated_obj.to_mesh(preserve_all_data_layers=True, depsgraph=depsgraph)

        if len(mesh.loop_triangles) == 0:
            mesh.calc_loop_triangles()

        # This should populate loop normals
        mesh.calc_normals_split()

        assert len(mesh.uv_layers) == 1, "Expected a mesh with exactly one uv layer"
        active_uvs = mesh.uv_layers.active.data

        if len(mesh.materials) > 0:
            material_loops_content = [{} for material in mesh.materials]
            material_loops = [[] for material in mesh.materials]
            material_tris = [[] for material in mesh.materials]
        else:
            # TODO: Special case for meshes that have a single material and use previous, simpler export instead?
            material_loops_content = [{}]
            material_loops = [[]]
            material_tris = [[]]

        for tri in mesh.loop_triangles:
            mi = tri.material_index
            loops = material_loops[mi]
            loops_content = material_loops_content[mi]
            tris = material_tris[mi]
            indices = []
            for li in tri.loops:
                if li not in loops_content:
                    l = mesh.loops[li]
                    if tri.use_smooth:
                        loops.append((li, None))
                    else:
                        loops.append((li, tri.normal))
                    loops_content[li] = len(loops) - 1
                indices.append(loops_content[li])
            # Blender uses different winding order
            tris.append((indices[0], indices[2], indices[1]))

        materials = mesh.materials if len(mesh.materials) > 0 else [None]
        for (i, material) in enumerate(materials):
            tris = material_tris[i]
            loops = material_loops[i]

            if material is not None:
                f.write(f"# {obj.name_full}:{material.name}\n")
            else:
                f.write(f"# {obj.name_full}\n")
            f.write(f"AttributeBegin\n")

            # TODO: Named materials, reuse
            if material is not None:
                _export_material(material, f)

            if not isclose3(pos, 0.0, 0.001):
                f.write(f"  Translate {fstr3(pos[0], pos[2], pos[1])}\n")
            if not math.isclose(rot_angle, 0.0, abs_tol=0.1):
                f.write(
                    f"  Rotate {fstr(-math.degrees(rot_angle))} {fstr3(rot_axis[0], rot_axis[2], rot_axis[1])}\n"
                )
            if not isclose3(scale, 1.0, 0.001):
                f.write(f"  Scale {fstr3(scale[0], scale[2], scale[1])}\n")

            if len(tris) > 100:
                if material is not None:
                    ply_path = f"plys/{mesh.name}_{material.name}.ply"
                else:
                    ply_path = f"plys/{mesh.name}.ply"
                if ply_path not in EXPORTED_PLY_MESHES:
                    with open(os.path.join(EXPORT_DIR, ply_path), "wb") as pf:
                        pf.write(b"ply\n")
                        pf.write(b"format binary_little_endian 1.0\n")
                        pf.write(f"element vertex {len(loops)}\n".encode())
                        pf.write(b"property float x\n")
                        pf.write(b"property float y\n")
                        pf.write(b"property float z\n")
                        pf.write(b"property float nx\n")
                        pf.write(b"property float ny\n")
                        pf.write(b"property float nz\n")
                        pf.write(b"property float u\n")
                        pf.write(b"property float v\n")
                        pf.write(f"element face {len(tris)}\n".encode())
                        pf.write(b"property list uchar int vertex_index\n")
                        pf.write(b"end_header\n")
                        for (li, face_n) in loops:
                            l = mesh.loops[li]

                            p = mesh.vertices[l.vertex_index].co

                            if face_n is not None:
                                n = face_n
                            else:
                                n = l.normal

                            uv = active_uvs[li].uv

                            pf.write(struct.pack("<f", p[0]))
                            pf.write(struct.pack("<f", p[2]))
                            pf.write(struct.pack("<f", p[1]))

                            pf.write(struct.pack("<f", n[0]))
                            pf.write(struct.pack("<f", n[2]))
                            pf.write(struct.pack("<f", n[1]))

                            pf.write(struct.pack("<f", uv[0]))
                            pf.write(struct.pack("<f", uv[1]))
                        for (i0, i1, i2) in tris:
                            pf.write(struct.pack("B", 3))
                            pf.write(struct.pack("<I", i0))
                            pf.write(struct.pack("<I", i1))
                            pf.write(struct.pack("<I", i2))
                    EXPORTED_PLY_MESHES.add(ply_path)

                f.write(f'  Shape "plymesh" "string filename" "{ply_path}"\n')
            else:
                f.write(f'  Shape "trianglemesh"\n')

                f.write(f'    "integer indices" [ ')
                for (i0, i1, i2) in tris:
                    f.write(f"{i0} {i1} {i2} ")
                f.write("]\n")

                f.write(f'    "point P" [ ')
                for (li, _) in loops:
                    p = mesh.vertices[mesh.loops[li].vertex_index].co
                    f.write(f"{fstr3(p[0], p[2], p[1])} ")
                f.write("]\n")

                f.write(f'    "normal N" [ ')
                for (li, face_n) in loops:
                    l = mesh.loops[li]
                    if face_n is not None:
                        n = face_n
                    else:
                        n = l.normal
                    f.write(f"{fstr3(n[0], n[2], n[1])} ")
                f.write("]\n")

                f.write(f'    "float uv" [ ')
                for (li, _) in loops:
                    uv = active_uvs[li].uv
                    f.write(f"{fstr(uv[0])} {fstr(uv[1])} ")
                f.write("]\n")

            f.write(f"AttributeEnd\n")
            f.write("\n")

    elif obj.type == "COLLECTION":
        _warn(f"'{obj.name_full}': Instanced collections not supported")

    for child in obj.children:
        _export_obj(depsgraph, child, f)


def _export_material(material, f):
    global SEEN_TEXTURES

    assert material is not None

    nodes = material.node_tree.nodes

    output = next((n for n in nodes if n.type == "OUTPUT_MATERIAL"), None)
    if output is None:
        _warn(f"{material.name_full}: No output in node tree. Using active material.")
        return

    if len(output.inputs["Surface"].links) == 0:
        _warn(
            f"{material.name_full}: No surface input connected to output node. Using active material."
        )
        return

    bsdf = output.inputs["Surface"].links[0].from_node
    if bsdf.type == "BSDF_DIFFUSE":
        color = bsdf.inputs["Color"]
        color_str = ""
        if len(color.links) > 0:
            assert len(color.links) == 1, "Unexpected second link"
            color_tex = color.links[0].from_node
            # TODO: assert color space? are pbrt input textures srgb?

            color_filepath = bpy.path.relpath(color_tex.image.filepath)
            # TODO: add 'Texture' line if unseen file, copy under tgt_path/textures
            if color_filepath not in SEEN_TEXTURES:
                assert (
                    color_filepath[0:2] == "//" and color_filepath[0:2] != "//.."
                ), f"{material.name}: Texture path is not relative '{color_filepath}'"

                color_abspath = bpy.path.abspath(color_filepath)
                color_relpath = color_filepath[2:]  # remove '//'

                color_outpath = os.path.join(
                    EXPORT_DIR, os.path.join("textures", color_relpath)
                )
                color_outdir = os.path.dirname(color_outpath)
                os.makedirs(color_outdir, exist_ok=True)
                shutil.copy(color_abspath, color_outpath)

                f.write(
                    f'Texture "{color_filepath}" "spectrum" "imagemap" "string filename" "textures/{color_relpath}"\n'
                )
                SEEN_TEXTURES.add(color_filepath)

            color_str = f'"texture Kd" "{color_filepath}"'
        else:
            color_str = f'"rgb Kd" [ {fstr3(color.default_value[0], color.default_value[1], color.default_value[2])} ]'

        roughness = bsdf.inputs["Roughness"]
        if len(roughness.links) > 0:
            _warn(
                f"{material.name_full}: Unexpected input connection to diffuse roughness. Using default roughness."
            )
            sigma = 0.0
        else:
            # This might not be 100% correct but it seems kind of close
            sigma = math.degrees(roughness.default_value)

        normal = bsdf.inputs["Normal"]
        if len(normal.links) > 0:
            _warn(f"{material.name_full}: Diffuse normal map is not supported.")

        f.write(f'Material "matte" {color_str} "float sigma" {sigma}\n')
    elif bsdf.type == "BSDF_GLASS":
        color = bsdf.inputs["Color"]
        if len(color.links) > 0:
            _warn(
                f"{material.name_full}: Unexpected input connection to glass color. Using default color."
            )
            color_value = (1.0, 1.0, 1.0)
        else:
            color_value = color.default_value

        roughness = bsdf.inputs["Roughness"]
        if len(roughness.links) > 0 or roughness.default_value > 0.001:
            _warn(f"{material.name_full}: Non-zero glass roughness is not supported.")

        ior = bsdf.inputs["IOR"]
        if len(ior.links) > 0:
            _warn(
                f"{material.name_full}: Unexpected input node in glass IOR. Using default value."
            )
            ior_value = 1.5
        else:
            ior_value = ior.default_value

        normal = bsdf.inputs["Normal"]
        if len(normal.links) > 0:
            _warn(f"{material.name_full}: Glass normal map is not supported.")

        f.write(f'Material "glass"\n')
        f.write(
            f'  "rgb Kr" [ {fstr3(color_value[0], color_value[1], color_value[2])} ]\n'
        )
        f.write(
            f'  "rgb Kt" [ {fstr3(color_value[0], color_value[1], color_value[2])} ]\n'
        )
        f.write(f'  "float eta" {fstr(ior_value)}\n')
    elif bsdf.type == "BSDF_GLOSSY":
        color = bsdf.inputs["Color"]
        if len(color.links) > 0:
            _warn(
                f"{material.name_full}: Unexpected input connection to glossy color. Using default color."
            )
            rs = (0.5, 0.5, 0.5)
        else:
            rs = color.default_value

        roughness = bsdf.inputs["Roughness"]
        if len(roughness.links) > 0:
            _warn(
                f"{material.name_full}: Unexpected input connection to glossy roughness. Using default roughness."
            )
            roughness_value = 0.5
        else:
            # This might not be 100% correct but it seems kind of close
            roughness_value = roughness.default_value

        normal = bsdf.inputs["Normal"]
        if len(normal.links) > 0:
            _warn(f"{material.name_full}: Glossy normal map is not supported.")

        f.write(
            f'Material "glossy" "rgb Rs" [ {fstr3(rs[0], rs[1], rs[2])} ] "float roughness" {roughness_value}\n'
        )


def fstr(v: float) -> str:
    return f"{v:.9g}"


def fstr3(v0: float, v1: float, v2: float) -> str:
    return f"{v0:.9g} {v1:.9g} {v2:.9g}"


def isclose3(v, ref: float, abs_tol: float) -> bool:
    return (
        math.isclose(v[0], ref, abs_tol=abs_tol)
        and math.isclose(v[1], ref, abs_tol=abs_tol)
        and math.isclose(v[2], ref, abs_tol=abs_tol)
    )

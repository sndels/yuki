import bpy
from bpy_extras.io_utils import ExportHelper
import logging
from mathutils import Vector
import math
import os
import traceback
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

    with open(filename, "w") as f:
        cam_obj = scene.camera
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
                len(bg_node.outputs[0].links) == 1
                and bg_node.outputs[0].links[0].to_socket.node.name == "World Output"
            ):
                bg_color = bg_node.inputs[0].default_value
                f.write(
                    f'LightSource "infinite" "rgb L" [ {fstr3(bg_color[0], bg_color[1], bg_color[2])} ]\n'
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

        f.write("WorldEnd\n")

        return True


def _error(msg: str):
    logger.error(msg)


def _warn(msg: str):
    global EXPORT_WARNINGS
    EXPORT_WARNINGS = True

    logger.warning(msg)


def _export_collection(depsgraph, collection, f: TextIO):
    for obj in collection.objects:
        _export_obj(depsgraph, obj, f)

    for collection in collection.children:
        _export_collection(depsgraph, collection, f)


def _export_obj(depsgraph, obj, f: TextIO):
    global MESH_COUNT
    global EXPORT_DIR

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
        else:
            logger.info(
                f"{obj.name_full}: Skipping unimplemented light type '{light.type}'"
            )
    elif obj.type == "MESH":
        pos, qrot, scale = trfn.decompose()
        rot_axis, rot_angle = qrot.to_axis_angle()

        # Get mesh with modifiers applied
        evaluated_obj = obj.evaluated_get(depsgraph)
        mesh = evaluated_obj.to_mesh(preserve_all_data_layers=True, depsgraph=depsgraph)

        if len(mesh.loop_triangles) == 0:
            mesh.calc_loop_triangles()

        if len(mesh.materials) > 0:
            material_vertices = [{} for material in mesh.materials]
            material_tris = [[] for material in mesh.materials]
        else:
            # TODO: Special case for meshes that have a single material and use previous, simpler export instead?
            material_vertices = [{}]
            material_tris = [[]]

        for tri in mesh.loop_triangles:
            mi = tri.material_index
            vertices = material_vertices[mi]
            tris = material_tris[mi]
            indices = []
            for vi in tri.vertices:
                v = mesh.vertices[vi]
                if v not in vertices:
                    vertices[v] = len(vertices)
                indices.append(vertices[v])
            # Blender uses different winding order
            tris.append((indices[0], indices[2], indices[1]))

        for (i, material) in enumerate(mesh.materials):
            tris = material_tris[i]
            vertices = material_vertices[i]

            f.write(f"# {obj.name_full}:{material.name}\n")
            f.write(f"AttributeBegin\n")

            # TODO: Named materials, reuse
            _export_material(material, f)

            if not isclose3(pos, 0.0, 0.001):
                f.write(f"  Translate {fstr3(pos[0], pos[2], pos[1])}\n")
            if not math.isclose(rot_angle, 0.0, abs_tol=0.1):
                f.write(
                    f"  Rotate {fstr(-math.degrees(rot_angle))} {fstr3(rot_axis[0], rot_axis[2], rot_axis[1])}\n"
                )
            if not isclose3(scale, 1.0, 0.001):
                f.write(f"  Scale {fstr3(scale[0], scale[2], scale[1])}\n")

            # TODO: (Binary) PLY instead of trimesh if mesh(part) is "large"
            f.write(f'  Shape "trianglemesh"\n')

            f.write(f'    "integer indices" [ ')
            for (i0, i1, i2) in tris:
                f.write(f"{i0} {i1} {i2} ")
            f.write("]\n")

            f.write(f'    "point P" [ ')
            for v in vertices:
                p = v.co
                f.write(f"{fstr3(p[0], p[2], p[1])} ")
            f.write("]\n")

            # TODO: Normals

            f.write(f"AttributeEnd\n")
            f.write("\n")

    elif obj.type == "COLLECTION":
        _warn(f"'{obj.name_full}': Instanced collections not supported")

    for child in obj.children:
        _export_obj(child, f)


def _export_material(material, f):
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
        if len(color.links) > 0:
            _warn(
                f"{material.name_full}: Unexpected input connection to diffuse color. Using default color."
            )
            color_value = (0.5, 0.5, 0.5)
        else:
            color_value = color.default_value

        roughness = bsdf.inputs["Roughness"]
        if len(roughness.links) > 0 or roughness.default_value > 0.001:
            _warn(f"{material.name_full}: Non-zero diffuse roughness is not supported.")

        normal = bsdf.inputs["Normal"]
        if len(normal.links) > 0:
            _warn(f"{material.name_full}: Diffuse normal map is not supported.")

        f.write(
            f'Material "matte" "rgb Kd" [ {fstr3(color_value[0], color_value[1], color_value[2])} ]\n'
        )
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
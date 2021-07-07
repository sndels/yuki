bl_info = {
    "name": "Yuki utils",
    "blender": (2, 92, 0),
    "category": "Development",
}

import bpy
import logging

logger = logging.getLogger("yuki_utils")
logger.setLevel(logging.INFO)


class YUKI_OT_ConvertAllMaterialsToDiffuse(bpy.types.Operator):
    """Convert all materials to Diffuse BSDF"""

    bl_idname = "yuki.convert_all_materials_to_diffuse"
    bl_label = "Convert all materials to Diffuse BSDF"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        del context  # unused

        for material in bpy.data.materials:
            if not material.node_tree:
                continue

            nodes = material.node_tree.nodes
            output = None
            color = None
            for node in nodes:
                if node.type == "OUTPUT_MATERIAL":
                    output = node
                    continue
                elif node.type == "BSDF_PRINCIPLED":
                    color = node.inputs["Base Color"].default_value
            if output is None:
                logger.info(f"No output node for material '{material.name}'")
                continue

            if color is None:
                logger.info(f"No diffuse color found for material '{material.name}'")
                continue

            for node in nodes:
                if node.type != "OUTPUT_MATERIAL":
                    nodes.remove(node)

            diffuse = nodes.new("ShaderNodeBsdfDiffuse")
            diffuse.inputs["Color"].default_value = color

            material.node_tree.links.new(
                output.inputs["Surface"], diffuse.outputs["BSDF"]
            )

        return {"FINISHED"}


def menu_func(self, context):
    self.layout.operator(YUKI_OT_ConvertAllMaterialsToDiffuse.bl_idname)


def register():
    bpy.utils.register_class(YUKI_OT_ConvertAllMaterialsToDiffuse)
    bpy.types.TOPBAR_MT_file_cleanup.append(menu_func)


def unregister():
    bpy.types.TOPBAR_MT_file_cleanup.remove(menu_func)
    bpy.utils.unregister_class(YUKI_OT_ConvertAllMaterialsToDiffuse)

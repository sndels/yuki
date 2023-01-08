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
            output_node = None
            color_node = None
            for node in nodes:
                if node.type == "OUTPUT_MATERIAL":
                    output_node = node
                    continue
                elif node.type == "BSDF_PRINCIPLED":
                    color_node = node.inputs["Base Color"]
            if output_node is None:
                logger.info(f"No output node for material '{material.name}'")
                continue

            if color_node is None:
                logger.info(f"No diffuse color found for material '{material.name}'")
                continue

            protected_nodes = set()
            protected_nodes.add(output_node)

            diffuse = nodes.new("ShaderNodeBsdfDiffuse")

            material.node_tree.links.new(
                output_node.inputs["Surface"], diffuse.outputs["BSDF"]
            )
            protected_nodes.add(diffuse)

            if (
                len(color_node.links) > 0
                and color_node.links[0].from_node.type == "TEX_IMAGE"
            ):
                protected_nodes.add(color_node.links[0].from_node)

                assert len(color_node.links) == 1, "Unexpected second link"
                material.node_tree.links.new(
                    diffuse.inputs["Color"],
                    color_node.links[0].from_node.outputs["Color"],
                )
            else:
                diffuse.inputs["Color"].default_value = color_node.default_value

            for node in nodes:
                if node not in protected_nodes:
                    nodes.remove(node)

        return {"FINISHED"}


def menu_func(self, context):
    self.layout.operator(YUKI_OT_ConvertAllMaterialsToDiffuse.bl_idname)


def register():
    bpy.utils.register_class(YUKI_OT_ConvertAllMaterialsToDiffuse)
    bpy.types.TOPBAR_MT_file_cleanup.append(menu_func)


def unregister():
    bpy.types.TOPBAR_MT_file_cleanup.remove(menu_func)
    bpy.utils.unregister_class(YUKI_OT_ConvertAllMaterialsToDiffuse)

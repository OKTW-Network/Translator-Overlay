# /// script
# requires-python = ">=3.10"
# dependencies = ["onnx", "onnxslim", "onnxruntime"]
# ///
"""Shrink the official PP-DocLayoutV3 ONNX for the layout line-merge mode.

Input: inference.onnx from https://huggingface.co/PaddlePaddle/PP-DocLayoutV3_onnx

* Drop the mask outputs (fetch_name_1/2). oar-ocr reads only the box tensor,
  but ORT still computed the 300x200x200 masks and copied them off the GPU.
* FP16 weights/activations, fp32 I/O. The box decode downstream of
  im_shape / scale_factor stays fp32: fp16 has 1 px steps above 1024.
  So does int->float index math: fp16 rounds TopK index 7499 to 7500,
  i.e. query 300 of 300 (WebGPU: "GatherND invalid index found").

DirectML on a 1080p frame: ~29 ms -> ~14 ms per run, same detections
(edges within ~2 px). CPU speed is unchanged.

    uv run scripts/optimize-doclayout.py inference.onnx pp-doclayoutv3_fp16.onnx
"""

from __future__ import annotations

import sys

import onnx
import onnxslim
from onnx import numpy_helper
from onnxruntime.transformers.float16 import convert_float_to_float16


def main(src: str, dst: str) -> None:
    m = onnx.load(src)
    keep = [o for o in m.graph.output if o.name == "fetch_name_0"]
    del m.graph.output[:]
    m.graph.output.extend(keep)
    m = onnxslim.slim(m)  # dead-code elimination + constant folding

    # Empty roi/scales -> omitted optional inputs; otherwise the fp16 pass casts
    # them and Resize sees both `scales` and `sizes`.
    empty = {i.name for i in m.graph.initializer if numpy_helper.to_array(i).size == 0}
    empty |= {n.output[0] for n in m.graph.node if n.op_type == "Constant" and numpy_helper.to_array(n.attribute[0].t).size == 0}
    for n in m.graph.node:
        if n.op_type == "Resize":
            for k in (1, 2):
                if k < len(n.input) and n.input[k] in empty:
                    n.input[k] = ""
    m = onnxslim.slim(m)

    # Index math (TopK index -> float -> /25 -> floor) also stays fp32: fp16 is
    # exact only up to 2048, flat query*class indices reach 7499.
    m = onnx.shape_inference.infer_shapes(m)
    dtype = {v.name: v.type.tensor_type.elem_type for v in [*m.graph.value_info, *m.graph.input, *m.graph.output]}
    dtype |= {i.name: i.data_type for i in m.graph.initializer}
    tainted, fp32_nodes = {"im_shape", "scale_factor"}, []
    for n in m.graph.node:  # topologically sorted
        int_to_float = n.op_type == "Cast" and dtype.get(n.input[0]) in (onnx.TensorProto.INT64, onnx.TensorProto.INT32)
        if int_to_float or tainted & set(n.input):
            fp32_nodes.append(n.name)
            # Stop at int outputs so index-driven Gathers don't drag the backbone along.
            tainted |= {o for o in n.output if dtype.get(o) == onnx.TensorProto.FLOAT}
    m = convert_float_to_float16(m, keep_io_types=True, node_block_list=fp32_nodes)
    kept = set(fp32_nodes)
    assert not any(
        n.name in kept and n.op_type == "Cast" and n.attribute[0].i == onnx.TensorProto.FLOAT16 for n in m.graph.node
    ), "blocked Cast was turned into fp16"
    print(f"{len(fp32_nodes)} nodes kept fp32")
    onnx.save(m, dst)


if __name__ == "__main__":
    main(*sys.argv[1:3])

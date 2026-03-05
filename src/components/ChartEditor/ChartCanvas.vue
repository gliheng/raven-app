<script setup lang="ts">
import { markRaw, nextTick } from 'vue';
import { ConnectionMode, VueFlow, Panel, useVueFlow, getRectOfNodes, MarkerType } from '@vue-flow/core';
import { Background } from '@vue-flow/background';
import { Controls } from '@vue-flow/controls';
import { MiniMap } from '@vue-flow/minimap';
import type { FlowExportObject, NodeChange, EdgeChange, NodeDimensionChange } from '@vue-flow/core';
import { nanoid } from 'nanoid';
import TextNode from './TextNode.vue';
import DropZoneBackground from './DropZoneBackground.vue';
import { useLayout } from './layout';
import useDragAndDrop from './useDnD';
import { isEqual } from 'lodash-es';

const flow = defineModel<FlowExportObject>({
  required: true,
});

const {
  onInit,
  onConnect,
  addEdges,
  addNodes,
  setNodes,
  onNodesChange,
  onEdgesChange,
  onNodeClick,
  toObject,
  fitView,
  applyNodeChanges,
  onViewportChange,
  fitBounds,
  getNodes,
  getEdges,
} = useVueFlow()

// Track nodes waiting for dimensions
const nodesWaitingForDimensions = new Set<string>();

function triggerChange() {
  const obj = toObject();
  if (!isEqual(obj, flow.value)) {
    flow.value = obj;
  }
}

onViewportChange((newViewport) => {
  triggerChange();
});

onNodesChange((changes: NodeChange[]) => {
  // Check for dimension changes on nodes we're waiting for
  for (const change of changes) {
    if (change.type === 'dimensions') {
      const dimChange = change as NodeDimensionChange;
      // Only focus if dimensions exist and are non-zero (actual dimensions received)
      if (
        nodesWaitingForDimensions.has(dimChange.id) &&
        dimChange.dimensions?.width &&
        dimChange.dimensions?.height &&
        dimChange.dimensions.width > 0 &&
        dimChange.dimensions.height > 0
      ) {
        nodesWaitingForDimensions.delete(dimChange.id);
        focusOnNode(dimChange.id);
      }
    }
  }

  triggerChange();
});

onEdgesChange((changes: EdgeChange[]) => {
  triggerChange();
});

onConnect((connection) => {
  addEdges(connection)
});

const nodeTypes = {
  text: markRaw(TextNode),
};

const { layout } = useLayout()

async function layoutGraph() {
  setNodes(layout(getNodes.value, getEdges.value))

  nextTick(() => {
    fitView()
  })
}

function addNode() {
  const id = nanoid();
  addNodes([{
    id,
    type: 'text',
    data: {},
    width: 320,
    height: 200,
    position: { x: 0, y: 0 },
  }]);

  const changes: NodeChange[] = [{
    id,
    type: 'select',
    selected: true,
  }];
  applyNodeChanges(changes);

  // Add to waiting list - will focus when dimensions are received
  nodesWaitingForDimensions.add(id);
}


function focusOnNode(nodeId: string) {
  const node = getNodes.value.find((node: any) => node.id === nodeId);
  if (!node) {
    return;
  }
  const rect = getRectOfNodes([node]);
  fitBounds(rect, {
    duration: 1000,
  })
}

function logToObject() {
  console.log(toObject())
}

const { onDragStart, onDragOver, onDrop, onDragLeave, isDragOver, isDragging } = useDragAndDrop()

defineExpose({
  getGraph() {
    return toObject();
  },
});
</script>

<template>
  <div class="flex size-full" @drop="onDrop">
    <VueFlow
      class="flow-chart"
      :nodes="flow.nodes"
      :edges="flow.edges"
      :min-zoom="0.2"
      :max-zoom="2"
      :node-types="nodeTypes"
      :default-edge-options="{
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 20,
          height: 20,
        },
      }"
      :connection-mode="ConnectionMode.Loose"
      :default-viewport="flow?.viewport"
      @dragover="onDragOver"
      @dragleave="onDragLeave"
    >
      <DropZoneBackground :is-drag-over="isDragOver" />
      <Background variant="dots" :gap="20" />
      <Controls />
      <Panel position="top-right">
        <UButton
          icon="i-heroicons-plus-circle-20-solid" 
          :draggable="true"
          @dragstart="onDragStart($event, 'text')"
          @click="addNode" />
      </Panel>
      <MiniMap />
    </VueFlow>
  </div>
</template>

<style lang="scss">
@import '@vue-flow/core/dist/style.css';
@import '@vue-flow/core/dist/theme-default.css';
@import '@vue-flow/controls/dist/style.css';
@import '@vue-flow/minimap/dist/style.css';

.vue-flow__minimap {
  transform: scale(75%);
  transform-origin: bottom right;
}

.flow-chart .vue-flow__node {
  --vf-node-color: var(--ui-color-neutral-500);
  --vf-handle: var(--ui-color-neutral-500);
  border: 1px solid var(--vf-node-color);
  border-radius: 0.4rem;
  background-color: var(--vf-node-bg);
}

.flow-chart .vue-flow__node.selected {
  --vf-node-color: var(--ui-color-primary-500);
  --vf-handle: var(--ui-color-primary-500);
}

.flow-chart .vue-flow__controls {
  display:flex;
  flex-wrap:wrap;
  justify-content:center
}

.flow-chart .vue-flow__controls .vue-flow__controls-button {
  border: none;
  border-right:1px solid #eee
}

.flow-chart .vue-flow__controls .vue-flow__controls-button svg {
  height:100%;
  width:100%
}
</style>

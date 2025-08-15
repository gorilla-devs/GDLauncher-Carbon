
import { createFileRoute } from '@tanstack/solid-router';
import { Tooltip } from '../../../../src';
import ComponentDemo from '../../components/ComponentDemo';

export const Route = createFileRoute('/components/tooltip')({
  component: TooltipPage,
});

function TooltipPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold mb-4" style={`color: rgb(var(--lightSlate-50))`}>Tooltip</h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Informational overlay that appears on hover or focus to provide additional context.
        </p>
      </div>

      <ComponentDemo
        title="Basic Tooltip"
        description="Simple tooltip that appears on hover"
      >
        <div class="flex items-center space-x-4">
          <Tooltip content="This is a helpful tooltip">
            <button 
              class="px-4 py-2 rounded-md transition-colors"
              style={`background-color: rgb(var(--primary-500)); color: white`}
            >
              Hover me
            </button>
          </Tooltip>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Tooltip Positions"
        description="Tooltips can appear in different positions"
      >
        <div class="flex items-center justify-center space-x-6">
          <Tooltip content="Top tooltip" position="top">
            <button 
              class="px-4 py-2 rounded-md transition-colors"
              style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
            >
              Top
            </button>
          </Tooltip>
          <Tooltip content="Bottom tooltip" position="bottom">
            <button 
              class="px-4 py-2 rounded-md transition-colors"
              style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
            >
              Bottom
            </button>
          </Tooltip>
          <Tooltip content="Left tooltip" position="left">
            <button 
              class="px-4 py-2 rounded-md transition-colors"
              style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
            >
              Left
            </button>
          </Tooltip>
          <Tooltip content="Right tooltip" position="right">
            <button 
              class="px-4 py-2 rounded-md transition-colors"
              style={`background-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-50))`}
            >
              Right
            </button>
          </Tooltip>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Rich Tooltip Content"
        description="Tooltips can contain rich content and formatting"
      >
        <Tooltip 
          content={
            <div>
              <strong>Rich Tooltip</strong>
              <br />
              <em>With formatting and multiple lines</em>
            </div>
          }
        >
          <button 
            class="px-4 py-2 rounded-md transition-colors"
            style={`background-color: rgb(var(--green-600)); color: white`}
          >
            Rich tooltip
          </button>
        </Tooltip>
      </ComponentDemo>
    </div>
  );
}

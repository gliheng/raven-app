import type { BaseMessage } from "@langchain/core/messages";
import { MessagesAnnotation, StateGraph, START, END } from "@langchain/langgraph";
import { ChatAnthropic } from "@langchain/anthropic";
import type { ConversationGraph, RuntimeConfig } from "./types.ts";

const CALL_MODEL_NODE_ID = "callModel";

interface GraphState {
  cwd: string;
  messages: BaseMessage[];
}

function createCallModelNode(model: ChatAnthropic): (state: GraphState) => Promise<GraphState> {
  return async (state: GraphState): Promise<GraphState> => {
    const response = await model.invoke(state.messages);

    return {
      cwd: state.cwd,
      // MessagesAnnotation appends node outputs onto the running conversation.
      messages: [response],
    };
  };
}

export async function createGraph(config: RuntimeConfig): Promise<ConversationGraph> {
  const model = new ChatAnthropic({
    model: config.model,
    apiKey: config.apiKey,
    anthropicApiUrl: config.baseUrl,
    temperature: 0,
  });

  const callModelNode = createCallModelNode(model);

  return new StateGraph(MessagesAnnotation)
    .addNode(CALL_MODEL_NODE_ID, callModelNode)
    .addEdge(START, CALL_MODEL_NODE_ID)
    .addEdge(CALL_MODEL_NODE_ID, END)
    .compile();
}

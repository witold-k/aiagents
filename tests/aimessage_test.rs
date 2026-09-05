#[cfg(test)]
mod tests {
    use serde_json::json;
    use aiagents::aimessage::{
        AIMessage,
        AIMessageId,
        AIMessageList,
        AIMessageListData,
        AIMessageType,
    };
    use aiagents::agenttools::aitooltype::AIToolType;
    use aiagents::generated_tasks::Tasks;

    fn message(
        id: usize,
        msgtype: AIMessageType,
        tooltype: AIToolType,
        data: &str,
    ) -> AIMessage {
        AIMessage::new(
            AIMessageId { val: id },
            msgtype,
            tooltype,
            data.to_string(),
        )
    }

    fn empty_list() -> AIMessageList {
        AIMessageList::new(AIMessageListData {
            messages: Vec::new(),
            message_id: AIMessageId { val: 0 },
            depth: 10,
            task_id: Tasks::FixCode,
            task_description: String::new(),
            subtask: Vec::new(),
            structureinfo: String::new(),
            files: Vec::new(),
            focus: String::new(),
            faults: None,
        })
    }

    // -----------------------------------------------------------------------
    // AIMessageType
    // -----------------------------------------------------------------------

    #[test]
    fn message_type_to_str() {
        assert_eq!(AIMessageType::System.to_str(), "system");
        assert_eq!(AIMessageType::User.to_str(), "user");
        assert_eq!(AIMessageType::Build.to_str(), "build");
        assert_eq!(AIMessageType::Tool.to_str(), "tool");
        assert_eq!(AIMessageType::Model.to_str(), "assistant");
    }

    #[test]
    fn message_type_equality() {
        assert_eq!(AIMessageType::User, AIMessageType::User);
        assert_ne!(AIMessageType::User, AIMessageType::Model);
    }

    // -----------------------------------------------------------------------
    // AIMessageId
    // -----------------------------------------------------------------------

    #[test]
    fn message_id_display() {
        assert_eq!(AIMessageId { val: 0 }.to_string(), "call_0");
        assert_eq!(AIMessageId { val: 1 }.to_string(), "call_1");
        assert_eq!(AIMessageId { val: 123 }.to_string(), "call_123");
    }

    // -----------------------------------------------------------------------
    // AIMessage
    // -----------------------------------------------------------------------

    #[test]
    fn message_constructor() {
        let msg = message(
            42,
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello",
        );

        assert_eq!(msg.message_id, AIMessageId { val: 42 });
        assert_eq!(msg.msgtype, AIMessageType::User);
        assert_eq!(msg.tooltype, AIToolType::LoadFile);
        assert_eq!(msg.data, "hello");
    }

    #[test]
    fn is_user_only_returns_true_for_user_messages() {
        assert!(message(
            1,
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello",
        )
        .is_user());

        assert!(!message(
            1,
            AIMessageType::System,
            AIToolType::LoadFile,
            "hello",
        )
        .is_user());

        assert!(!message(
            1,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "hello",
        )
        .is_user());

        assert!(!message(
            1,
            AIMessageType::Tool,
            AIToolType::LoadFile,
            "hello",
        )
        .is_user());

        assert!(!message(
            1,
            AIMessageType::Build,
            AIToolType::LoadFile,
            "hello",
        )
        .is_user());
    }

    #[test]
    fn system_message_to_json() {
        let msg = message(
            1,
            AIMessageType::System,
            AIToolType::LoadFile,
            "system prompt",
        );

        assert_eq!(
            msg.to_json(),
            json!({
                "role": "system",
                "content": "system prompt"
            })
        );
    }

    #[test]
    fn user_message_to_json() {
        let msg = message(
            2,
            AIMessageType::User,
            AIToolType::LoadFile,
            "user prompt",
        );

        assert_eq!(
            msg.to_json(),
            json!({
                "role": "user",
                "content": "user prompt"
            })
        );
    }

    #[test]
    fn build_message_to_json() {
        let msg = message(
            3,
            AIMessageType::Build,
            AIToolType::LoadFile,
            "build information",
        );

        assert_eq!(
            msg.to_json(),
            json!({
                "role": "user",
                "content": "build information"
            })
        );
    }

    #[test]
    fn model_message_to_json() {
        let msg = message(
            4,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "assistant response",
        );

        assert_eq!(
            msg.to_json(),
            json!({
                "role": "assistant",
                "assistant_call_id": "call_4",
                "content": "assistant response"
            })
        );
    }

    #[test]
    fn tool_message_to_json() {
        let msg = message(
            5,
            AIMessageType::Tool,
            AIToolType::LoadFile,
            "file contents",
        );

        assert_eq!(
            msg.to_json(),
            json!({
                "role": "user",
                "tool_call_id": "call_5",
                "content": "file contents"
            })
        );
    }

    #[test]
    fn message_to_short_string() {
        let msg = message(
            7,
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello world",
        );

        let result = msg.to_short_string();

        assert!(result.contains("id: call_7"));
        assert!(result.contains("role: user"));
        assert!(result.contains("tooltype: LoadFile"));
        assert!(result.contains("content: hello world"));
    }

    // -----------------------------------------------------------------------
    // AIMessageList::new
    // -----------------------------------------------------------------------

    #[test]
    fn list_new_copies_data_and_initializes_note() {
        let data = AIMessageListData {
            messages: vec![
                message(
                    1,
                    AIMessageType::User,
                    AIToolType::LoadFile,
                    "hello",
                ),
            ],
            message_id: AIMessageId { val: 10 },
            depth: 5,
            task_id: Tasks::FixCode,
            task_description: "task description".to_string(),
            subtask: vec!["subtask 1".to_string()],
            structureinfo: "AST".to_string(),
            files: Vec::new(),
            focus: "focus".to_string(),
            faults: Some("fault".to_string()),
        };

        let list = AIMessageList::new(data);

        assert_eq!(list.messages.len(), 1);
        assert_eq!(list.message_id.val, 10);
        assert_eq!(list.depth, 5);
        assert_eq!(list.task_description, "task description");
        assert_eq!(list.subtask, vec!["subtask 1"]);
        assert_eq!(list.structureinfo, "AST");
        assert!(list.files.is_empty());
        assert_eq!(list.note, "");
        assert_eq!(list.focus, "focus");
        assert_eq!(list.faults.as_deref(), Some("fault"));
    }

    // -----------------------------------------------------------------------
    // inc_messageid
    // -----------------------------------------------------------------------

    #[test]
    fn inc_messageid_increments_and_returns_new_id() {
        let mut list = empty_list();

        assert_eq!(list.message_id.val, 0);

        let id1 = list.inc_messageid();
        assert_eq!(id1.val, 1);
        assert_eq!(list.message_id.val, 1);

        let id2 = list.inc_messageid();
        assert_eq!(id2.val, 2);
        assert_eq!(list.message_id.val, 2);
    }

    // -----------------------------------------------------------------------
    // append
    // -----------------------------------------------------------------------

    #[test]
    fn append_adds_message() {
        let mut list = empty_list();

        list.append(
            AIMessageId { val: 1 },
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello",
        );

        assert_eq!(list.messages.len(), 1);
        assert_eq!(list.messages[0].message_id.val, 1);
        assert_eq!(list.messages[0].msgtype, AIMessageType::User);
        assert_eq!(list.messages[0].data, "hello");
    }

    #[test]
    fn append_load_file_references_matching_model_request() {
        let mut list = empty_list();

        // Model requests a particular file.
        list.messages.push(message(
            1,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "src/main.rs",
        ));

        // Tool result for that request.
        list.append(
            AIMessageId { val: 2 },
            AIMessageType::Tool,
            AIToolType::LoadFile,
            "src/main.rs",
        );

        assert_eq!(list.messages.len(), 2);

        assert_eq!(list.messages[0].data, "src/main.rs");
        assert_eq!(list.messages[1].data, "REF: call_2");
    }

    #[test]
    fn append_ast_references_matching_model_request() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::Model,
            AIToolType::Ast,
            "src/main.rs",
        ));

        list.append(
            AIMessageId { val: 2 },
            AIMessageType::Tool,
            AIToolType::Ast,
            "src/main.rs",
        );

        assert_eq!(list.messages[1].data, "REF: call_2");
    }

    #[test]
    fn append_does_not_reference_unrelated_model_request() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "src/other.rs",
        ));

        list.append(
            AIMessageId { val: 2 },
            AIMessageType::Tool,
            AIToolType::LoadFile,
            "src/main.rs",
        );

        assert_eq!(list.messages.len(), 2);
        assert_eq!(list.messages[0].data, "src/other.rs");
        assert_eq!(list.messages[1].data, "src/main.rs");
    }

    #[test]
    fn append_does_not_reference_model_with_different_tool_type() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::Model,
            AIToolType::Ast,
            "src/main.rs",
        ));

        list.append(
            AIMessageId { val: 2 },
            AIMessageType::Tool,
            AIToolType::LoadFile,
            "src/main.rs",
        );

        assert_eq!(list.messages[0].data, "src/main.rs");
        assert_eq!(list.messages[1].data, "src/main.rs");
    }

    #[test]
    fn append_only_references_the_next_message_after_matching_request() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "src/main.rs",
        ));

        list.messages.push(message(
            99,
            AIMessageType::User,
            AIToolType::LoadFile,
            "unrelated",
        ));

        list.append(
            AIMessageId { val: 2 },
            AIMessageType::Tool,
            AIToolType::LoadFile,
            "src/main.rs",
        );

        // The first message after the matching model request is replaced.
        assert_eq!(list.messages[1].data, "REF: call_2");

        // The appended tool message itself remains unchanged.
        assert_eq!(list.messages[2].data, "src/main.rs");
    }

    // -----------------------------------------------------------------------
    // clear
    // -----------------------------------------------------------------------

    #[test]
    fn clear_removes_messages_and_faults() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello",
        ));
        list.faults = Some("something went wrong".to_string());

        list.clear();

        assert!(list.messages.is_empty());
        assert!(list.faults.is_none());
    }

    #[test]
    fn clear_does_not_modify_other_fields() {
        let mut list = empty_list();

        list.depth = 42;
        list.task_description = "task".to_string();
        list.focus = "focus".to_string();
        list.note = "note".to_string();

        list.messages.push(message(
            1,
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello",
        ));
        list.faults = Some("fault".to_string());

        list.clear();

        assert_eq!(list.depth, 42);
        assert_eq!(list.task_description, "task");
        assert_eq!(list.focus, "focus");
        assert_eq!(list.note, "note");
    }

    // -----------------------------------------------------------------------
    // cut_to_depth
    // -----------------------------------------------------------------------

    #[test]
    fn cut_to_depth_does_nothing_when_within_depth() {
        let mut list = empty_list();
        list.depth = 3;

        for id in 1..=3 {
            list.messages.push(message(
                id,
                AIMessageType::User,
                AIToolType::LoadFile,
                &format!("message {id}"),
            ));
        }

        list.cut_to_depth();

        assert_eq!(list.messages.len(), 3);
        assert_eq!(list.messages[0].message_id.val, 1);
    }

    #[test]
    fn cut_to_depth_keeps_only_last_messages() {
        let mut list = empty_list();
        list.depth = 2;

        for id in 1..=5 {
            list.messages.push(message(
                id,
                AIMessageType::User,
                AIToolType::LoadFile,
                &format!("message {id}"),
            ));
        }

        list.cut_to_depth();

        assert_eq!(list.messages.len(), 2);
        assert_eq!(list.messages[0].message_id.val, 4);
        assert_eq!(list.messages[1].message_id.val, 5);
    }

    #[test]
    fn cut_to_depth_with_zero_depth_clears_messages() {
        let mut list = empty_list();
        list.depth = 0;

        list.messages.push(message(
            1,
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello",
        ));

        list.cut_to_depth();

        assert!(list.messages.is_empty());
    }

    // -----------------------------------------------------------------------
    // remove_type
    // -----------------------------------------------------------------------

    #[test]
    fn remove_type_removes_only_requested_type() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::System,
            AIToolType::LoadFile,
            "system",
        ));
        list.messages.push(message(
            2,
            AIMessageType::User,
            AIToolType::LoadFile,
            "user",
        ));
        list.messages.push(message(
            3,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "model",
        ));
        list.messages.push(message(
            4,
            AIMessageType::Tool,
            AIToolType::LoadFile,
            "tool",
        ));

        list.remove_type(AIMessageType::Model);

        assert_eq!(list.messages.len(), 3);
        assert_eq!(list.messages[0].msgtype, AIMessageType::System);
        assert_eq!(list.messages[1].msgtype, AIMessageType::User);
        assert_eq!(list.messages[2].msgtype, AIMessageType::Tool);
    }

    #[test]
    fn remove_type_does_nothing_when_type_is_absent() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello",
        ));

        list.remove_type(AIMessageType::Model);

        assert_eq!(list.messages.len(), 1);
        assert_eq!(list.messages[0].data, "hello");
    }

    // -----------------------------------------------------------------------
    // AIMessageList::to_json
    // -----------------------------------------------------------------------

    #[test]
    fn list_to_json_without_messages_creates_initial_user_message() {
        let mut list = empty_list();
        list.task_description = "Do something".to_string();

        let result = list.to_json();

        assert_eq!(
            result,
            json!([
                {
                    "role": "user",
                    "content": "Do something\n"
                }
            ])
        );
    }

    #[test]
    fn list_to_json_includes_subtasks() {
        let mut list = empty_list();
        list.task_description = "Main task".to_string();
        list.subtask = vec![
            "Subtask A".to_string(),
            "Subtask B".to_string(),
        ];

        let result = list.to_json();

        assert_eq!(
            result,
            json!([
                {
                    "role": "user",
                    "content": "Main task\nSubtask A\nSubtask B\n"
                }
            ])
        );
    }

    #[test]
    fn list_to_json_includes_structure_info() {
        let mut list = empty_list();
        list.task_description = "Task".to_string();
        list.structureinfo = "fn main() {}".to_string();

        let result = list.to_json();

        assert_eq!(
            result,
            json!([
                {
                    "role": "user",
                    "content":
                        "Task\n=== INFO/AST ===\nfn main() {}\n"
                }
            ])
        );
    }

    #[test]
    fn list_to_json_includes_faults() {
        let mut list = empty_list();
        list.task_description = "Task".to_string();
        list.faults = Some("Build failed".to_string());

        let result = list.to_json();

        assert_eq!(
            result,
            json!([
                {
                    "role": "user",
                    "content": "Task\nBuild failed\n"
                }
            ])
        );
    }

    #[test]
    fn list_to_json_includes_note() {
        let mut list = empty_list();
        list.task_description = "Task".to_string();
        list.note = "Important note".to_string();

        let result = list.to_json();

        assert_eq!(
            result,
            json!([
                {
                    "role": "user",
                    "content": "Task\n=== NOTE ===\nImportant note\n"
                }
            ])
        );
    }

    #[test]
    fn list_to_json_includes_focus() {
        let mut list = empty_list();
        list.task_description = "Task".to_string();
        list.focus = "Focus area".to_string();

        let result = list.to_json();

        assert_eq!(
            result,
            json!([
                {
                    "role": "user",
                    "content": "Task\n=== FOCUS ===\nFocus area\n"
                }
            ])
        );
    }

    #[test]
    fn list_to_json_prepends_context_to_first_user_message() {
        let mut list = empty_list();

        list.task_description = "Task description".to_string();
        list.messages.push(message(
            1,
            AIMessageType::User,
            AIToolType::LoadFile,
            "original user message",
        ));

        list.messages.push(message(
            2,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "assistant response",
        ));

        let result = list.to_json();

        assert_eq!(
            result,
            json!([
                {
                    "role": "user",
                    "content": "Task description\noriginal user message"
                },
                {
                    "role": "assistant",
                    "assistant_call_id": "call_2",
                    "content": "assistant response"
                }
            ])
        );
    }

    #[test]
    fn list_to_json_adds_context_as_separate_user_message_when_first_is_not_user() {
        let mut list = empty_list();

        list.task_description = "Task description".to_string();

        list.messages.push(message(
            1,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "assistant response",
        ));

        let result = list.to_json();

        assert_eq!(
            result,
            json!([
                {
                    "role": "user",
                    "content": "Task description\n"
                },
                {
                    "role": "assistant",
                    "assistant_call_id": "call_1",
                    "content": "assistant response"
                }
            ])
        );
    }

    // -----------------------------------------------------------------------
    // AIMessageList::to_short_string
    // -----------------------------------------------------------------------

    #[test]
    fn list_to_short_string_includes_subtasks() {
        let mut list = empty_list();

        list.subtask = vec![
            "subtask A".to_string(),
            "subtask B".to_string(),
        ];

        let result = list.to_short_string();

        assert!(result.contains("subtask A\n"));
        assert!(result.contains("subtask B\n"));
    }

    #[test]
    fn list_to_short_string_uses_type_1_for_first_user_message() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::User,
            AIToolType::LoadFile,
            "hello",
        ));

        let result = list.to_short_string();

        assert!(result.contains("==1= id: call_1"));
        assert!(result.contains("role: user"));
        assert!(result.contains("content: hello"));
    }

    #[test]
    fn list_to_short_string_uses_type_2_when_first_message_is_not_user() {
        let mut list = empty_list();

        list.messages.push(message(
            1,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "hello",
        ));

        let result = list.to_short_string();

        assert!(result.contains("==2= id: call_1"));
        assert!(result.contains("role: assistant"));
        assert!(result.contains("content: hello"));
    }

    #[test]
    fn list_to_short_string_includes_faults_note_and_focus() {
        let mut list = empty_list();

        list.faults = Some("fault".to_string());
        list.note = "note".to_string();
        list.focus = "focus".to_string();

        let result = list.to_short_string();

        assert!(result.contains("fault\n"));
        assert!(result.contains("=== NOTE ===\nnote\n"));
        assert!(result.contains("=== FOCUS ===\nfocus\n"));
    }

    // -----------------------------------------------------------------------
    // Serialization
    // -----------------------------------------------------------------------

    #[test]
    fn message_id_serializes_and_deserializes() {
        let id = AIMessageId { val: 123 };

        let serialized = serde_json::to_string(&id).unwrap();
        let deserialized: AIMessageId =
            serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, id);
    }

    #[test]
    fn message_type_serializes_and_deserializes() {
        for msgtype in [
            AIMessageType::System,
            AIMessageType::User,
            AIMessageType::Build,
            AIMessageType::Tool,
            AIMessageType::Model,
        ] {
            let serialized = serde_json::to_string(&msgtype).unwrap();
            let deserialized: AIMessageType =
                serde_json::from_str(&serialized).unwrap();

            assert_eq!(deserialized, msgtype);
        }
    }

    #[test]
    fn message_serializes_and_deserializes() {
        let original = message(
            42,
            AIMessageType::Model,
            AIToolType::LoadFile,
            "hello",
        );

        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: AIMessage =
            serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.message_id, original.message_id);
        assert_eq!(deserialized.msgtype, original.msgtype);
        assert_eq!(deserialized.tooltype, original.tooltype);
        assert_eq!(deserialized.data, original.data);
    }
}


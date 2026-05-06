use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // General
    Quit,
    Sync,
    NextPane,
    PrevPane,
    Search,
    Help,
    Cancel,

    // Task Addition
    QuickAdd,
    InsertTaskBelow,
    InsertTaskAbove,
    AddTaskBottom,
    AddTaskTop,
    AddSubtask,

    // Task Editing
    EditDescription,
    EditNotes,
    ToggleNotes,
    AddTag,
    ToggleSubtasks,
    ToggleExpand,
    EditDate,
    AssignTask,
    WriteComment,
    ToggleCompleted, // 'c'
    CompleteAndArchive, // 'Shift-C'
    DeleteTask,
    SnoozeTask,
    GetTaskLink,
    ViewComments,
    ViewActivity,
    GoToTaskOverview,

    // Task Movement
    MoveToPosition(u8),
    MoveToPositionNoScroll(u8),
    MoveToEnd,
    MoveTaskUp,
    MoveTaskDown,
    IndentTask,
    UnindentTask,
    MoveToQueue,
    CopyToQueue,

    // Queue Management
    MakeNewQueue,
    MakeNewCategory,
    ToggleMyQueues,
    ToggleSharedQueues,
    ShareQueue,
    ViewQueueDetails,
    ViewQueueActivity,
    PrintQueue,
    ToggleFullscreen,

    // Queue Navigation
    GoToInbox,
    GoToTrash,
    GoToDefaultQueue,
    GoToQueue,
    GoToActiveTasks,
    GoToArchivedTasks,
    GoBack,
    GoNext,

    // Navigation (Generic)
    MoveUp,
    MoveDown,
    Select,

    // Bulk / Global Toggles
    ToggleAllNotes,
    ToggleAllTags,
    ToggleAllSubtasks,
    ToggleAllAssignments,
    ToggleAllAttachments,
    ToggleAllCreatedDates,
    ToggleEverything,
}

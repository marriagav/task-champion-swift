import RustXcframework
public func new_replica_in_memory() -> Replica {
    Replica(ptr: __swift_bridge__$new_replica_in_memory())
}
public func new_replica_on_disk<GenericIntoRustString: IntoRustString>(_ taskdb_dir: GenericIntoRustString, _ create_if_missing: Bool, _ read_write: Bool) -> Replica {
    Replica(ptr: __swift_bridge__$new_replica_on_disk({ let rustString = taskdb_dir.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), create_if_missing, read_write))
}
public func new_operations() -> RustVec<Operation> {
    RustVec(ptr: __swift_bridge__$new_operations())
}
public func create_task(_ uuid: Uuid, _ ops: RustVec<Operation>) -> RustVec<Operation> {
    RustVec(ptr: __swift_bridge__$create_task({uuid.isOwned = false; return uuid.ptr;}(), { let val = ops; val.isOwned = false; return val.ptr }()))
}
public func uuid_v4() -> Uuid {
    Uuid(ptr: __swift_bridge__$uuid_v4())
}

public class Replica: ReplicaRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Replica$_free(ptr)
        }
    }
}
public class ReplicaRefMut: ReplicaRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
extension ReplicaRefMut {
    public func all_task_data() -> Optional<RustVec<TaskData>> {
        { let val = __swift_bridge__$Replica$all_task_data(ptr); if val != nil { return RustVec(ptr: val!) } else { return nil } }()
    }

    public func commit_operations(_ ops: RustVec<Operation>) {
        __swift_bridge__$Replica$commit_operations(ptr, { let val = ops; val.isOwned = false; return val.ptr }())
    }
}
public class ReplicaRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension Replica: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Replica$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Replica$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Replica) {
        __swift_bridge__$Vec_Replica$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Replica$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Replica(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ReplicaRef> {
        let pointer = __swift_bridge__$Vec_Replica$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ReplicaRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<ReplicaRefMut> {
        let pointer = __swift_bridge__$Vec_Replica$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return ReplicaRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<ReplicaRef> {
        UnsafePointer<ReplicaRef>(OpaquePointer(__swift_bridge__$Vec_Replica$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Replica$len(vecPtr)
    }
}


public class Operation: OperationRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Operation$_free(ptr)
        }
    }
}
public class OperationRefMut: OperationRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class OperationRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension Operation: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Operation$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Operation$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Operation) {
        __swift_bridge__$Vec_Operation$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Operation$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Operation(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OperationRef> {
        let pointer = __swift_bridge__$Vec_Operation$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OperationRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<OperationRefMut> {
        let pointer = __swift_bridge__$Vec_Operation$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return OperationRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<OperationRef> {
        UnsafePointer<OperationRef>(OpaquePointer(__swift_bridge__$Vec_Operation$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Operation$len(vecPtr)
    }
}


public class TaskData: TaskDataRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$TaskData$_free(ptr)
        }
    }
}
public class TaskDataRefMut: TaskDataRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
extension TaskDataRefMut {
    public func update<GenericIntoRustString: IntoRustString>(_ property: GenericIntoRustString, _ value: GenericIntoRustString, _ ops: RustVec<Operation>) {
        __swift_bridge__$TaskData$update(ptr, { let rustString = property.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let rustString = value.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let val = ops; val.isOwned = false; return val.ptr }())
    }

    public func update_remove<GenericIntoRustString: IntoRustString>(_ property: GenericIntoRustString, _ ops: RustVec<Operation>) {
        __swift_bridge__$TaskData$update_remove(ptr, { let rustString = property.intoRustString(); rustString.isOwned = false; return rustString.ptr }(), { let val = ops; val.isOwned = false; return val.ptr }())
    }

    public func delete_task(_ ops: RustVec<Operation>) {
        __swift_bridge__$TaskData$delete_task(ptr, { let val = ops; val.isOwned = false; return val.ptr }())
    }
}
public class TaskDataRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension TaskDataRef {
    public func get_uuid() -> Uuid {
        Uuid(ptr: __swift_bridge__$TaskData$get_uuid(ptr))
    }

    public func has<GenericIntoRustString: IntoRustString>(_ property: GenericIntoRustString) -> Bool {
        __swift_bridge__$TaskData$has(ptr, { let rustString = property.intoRustString(); rustString.isOwned = false; return rustString.ptr }())
    }

    public func properties() -> RustVec<RustString> {
        RustVec(ptr: __swift_bridge__$TaskData$properties(ptr))
    }
}
extension TaskData: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_TaskData$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_TaskData$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: TaskData) {
        __swift_bridge__$Vec_TaskData$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_TaskData$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (TaskData(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TaskDataRef> {
        let pointer = __swift_bridge__$Vec_TaskData$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TaskDataRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<TaskDataRefMut> {
        let pointer = __swift_bridge__$Vec_TaskData$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return TaskDataRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<TaskDataRef> {
        UnsafePointer<TaskDataRef>(OpaquePointer(__swift_bridge__$Vec_TaskData$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_TaskData$len(vecPtr)
    }
}


public class Uuid: UuidRefMut {
    var isOwned: Bool = true

    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }

    deinit {
        if isOwned {
            __swift_bridge__$Uuid$_free(ptr)
        }
    }
}
public class UuidRefMut: UuidRef {
    public override init(ptr: UnsafeMutableRawPointer) {
        super.init(ptr: ptr)
    }
}
public class UuidRef {
    var ptr: UnsafeMutableRawPointer

    public init(ptr: UnsafeMutableRawPointer) {
        self.ptr = ptr
    }
}
extension Uuid: Vectorizable {
    public static func vecOfSelfNew() -> UnsafeMutableRawPointer {
        __swift_bridge__$Vec_Uuid$new()
    }

    public static func vecOfSelfFree(vecPtr: UnsafeMutableRawPointer) {
        __swift_bridge__$Vec_Uuid$drop(vecPtr)
    }

    public static func vecOfSelfPush(vecPtr: UnsafeMutableRawPointer, value: Uuid) {
        __swift_bridge__$Vec_Uuid$push(vecPtr, {value.isOwned = false; return value.ptr;}())
    }

    public static func vecOfSelfPop(vecPtr: UnsafeMutableRawPointer) -> Optional<Self> {
        let pointer = __swift_bridge__$Vec_Uuid$pop(vecPtr)
        if pointer == nil {
            return nil
        } else {
            return (Uuid(ptr: pointer!) as! Self)
        }
    }

    public static func vecOfSelfGet(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<UuidRef> {
        let pointer = __swift_bridge__$Vec_Uuid$get(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return UuidRef(ptr: pointer!)
        }
    }

    public static func vecOfSelfGetMut(vecPtr: UnsafeMutableRawPointer, index: UInt) -> Optional<UuidRefMut> {
        let pointer = __swift_bridge__$Vec_Uuid$get_mut(vecPtr, index)
        if pointer == nil {
            return nil
        } else {
            return UuidRefMut(ptr: pointer!)
        }
    }

    public static func vecOfSelfAsPtr(vecPtr: UnsafeMutableRawPointer) -> UnsafePointer<UuidRef> {
        UnsafePointer<UuidRef>(OpaquePointer(__swift_bridge__$Vec_Uuid$as_ptr(vecPtr)))
    }

    public static func vecOfSelfLen(vecPtr: UnsafeMutableRawPointer) -> UInt {
        __swift_bridge__$Vec_Uuid$len(vecPtr)
    }
}

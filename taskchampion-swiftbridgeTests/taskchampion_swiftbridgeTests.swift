//
//  taskchampion_swiftbridgeTests.swift
//  taskchampion-swiftbridgeTests
//
//  Created by Cameron Johnson on 3/12/25.
//

import Taskchampion
import Testing
@testable import taskchampion_swiftbridge

struct taskchampion_swiftbridgeTests {

    @Test func example() async throws {
        let replica = Taskchampion.new_replica_in_memory();
        var tasks = replica.all_task_data()!;
        #expect(tasks.len() == 0);
        
        var ops = Taskchampion.new_operations();
        #expect(ops.len() == 0);
        ops = Taskchampion.create_task(Taskchampion.uuid_v4(), ops);
        #expect(ops.len() == 1);
        
        replica.commit_operations(ops);
        tasks = replica.all_task_data()!;
        #expect(tasks.len() == 1);
    }
    

}

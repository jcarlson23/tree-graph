
#[cfg(test)]
mod tests {

    use super::*;
    use crate::{ASTGraph,SerializableGraph};
    use tree_sitter_cpp;
    use tree_sitter_fortran;
    use crate::geometry::{GNode,GPoint,GRange};
    use tree_sitter::{Parser,TreeCursor,Node};
    use std::fs::File;
    use std::collections::HashSet;
    use bincode::{serialize_into, deserialize_from};

    const FORTRAN_CODE:&str = r#"
    program combined_program
        implicit none

        ! Call the functions
        call pointer_dangling()
        call type_conversion()

    end program combined_program

    ! Function to demonstrate a dangling pointer
    subroutine pointer_dangling()
        implicit none
        integer, pointer :: p
        integer, target :: t

        allocate(p)
        p => t
        deallocate(p) ! p is now freed, and so dangling

        ! Print statement commented to avoid runtime error from dangling pointer
        ! print *, p ! dereference
        print *, "Dangling pointer scenario demonstrated."
    end subroutine pointer_dangling

    ! Function to illustrate type conversion issues
    subroutine type_conversion()
        implicit none
        integer :: i
        real :: r

        ! Potential overflow for 32-bit integer
        i = 2**31
        print *, "Integer potential overflow value: ", i

        ! Division by zero scenario
        r = 1.0 / 0.0
        print *, "Result of division by zero: ", r
    end subroutine type_conversion
    "#;

    #[test]
    fn correct_number_of_subgraphs() {
       
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_fortran::language()).expect("Error loading Fortran grammar");

        let tree = parser.parse(FORTRAN_CODE, None).unwrap();

        let mut ast_graph = ASTGraph::new(FORTRAN_CODE.to_string());
        ast_graph.build_from_tree(&tree);

        // Extract subgraphs for each function
        let fortran_subroutine_kind:u16 = 229;
        let fortran_program_kind:u16 = 212;
        let mut fortran_types_to_split = HashSet::new();

        fortran_types_to_split.insert(fortran_subroutine_kind);
        fortran_types_to_split.insert(fortran_program_kind);

        let function_subgraphs = ast_graph.extract_subgraphs( fortran_types_to_split );
        // we should get 2 subroutines and 1 program = 3 subgraphs
        assert_eq!(function_subgraphs.len(),3);
    }

    #[test] 
    fn count_nodes_in_subgraphs() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_fortran::language()).expect("Error loading Fortran grammar");

        let tree = parser.parse(FORTRAN_CODE, None).unwrap();

        let mut ast_graph = ASTGraph::new(FORTRAN_CODE.to_string());
        ast_graph.build_from_tree(&tree);

        // Extract subgraphs for each function
        let fortran_subroutine_kind:u16 = 229;
        let fortran_program_kind:u16 = 212;
        let mut fortran_types_to_split = HashSet::new();

        fortran_types_to_split.insert(fortran_subroutine_kind);
        fortran_types_to_split.insert(fortran_program_kind);

        let total_nodes = ast_graph.node_count();
        let mut subgraph_counts = Vec::new();

        let function_subgraphs = ast_graph.extract_subgraphs( fortran_types_to_split );
        for subgraph in function_subgraphs.iter() {
            let count = subgraph.node_count();
            subgraph_counts.push(count);
        }

        let number_of_subgraphs = function_subgraphs.len();

        let subgraph_totals:usize = subgraph_counts.iter().sum();
        assert_eq!(subgraph_totals + number_of_subgraphs, total_nodes);

    }

    use std::fs;

    // utility function for testing serializing graphs
    fn check_file_existence_and_nonempty(file_path: &str) -> std::io::Result<bool> {
        // Check if the file exists
        let metadata = fs::metadata(file_path)?;
        
        // Check if the file is not empty
        let file_size = metadata.len();

        Ok(file_size > 0)
    }

    #[test]
    fn serialize_ast_graph() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_fortran::language()).expect("Error loading Fortran grammar");

        let tree = parser.parse(FORTRAN_CODE, None).unwrap();

        let mut ast_graph = ASTGraph::new(FORTRAN_CODE.to_string());
        ast_graph.build_from_tree(&tree);
        let file = File::create("graph.bin").expect("Failed to open a file");
        let serializable_graph = ast_graph.to_serializable();
        let result = serialize_into(file, &serializable_graph);
        match result {
            Ok(_) => {
                assert!(true);
            },
            Err(_err) => {
                assert!(false);
            }
        }

        let file_exists = check_file_existence_and_nonempty("graph.bin");
        match file_exists {
            Ok(true) => {
                assert!(true);
            },
            Ok(false) => {
                assert!(false);
            }
            Err(_err) => {
                assert!(false);
            }
        }
    }

    #[test] 
    fn simple_deserialization_test()
    {
        // Create a sample ASTGraph for testing
        let mut ast_graph = ASTGraph::new("testing".to_string());
        let a = ast_graph.graph.add_node(GNode { id: 1, kind_id: 1, range: GRange { start_byte: 0, end_byte: 5, start_point: GPoint { row: 1, column: 1 }, end_point: GPoint { row: 2, column: 5 } } });
        let b = ast_graph.graph.add_node(GNode { id: 2, kind_id: 3, range: GRange { start_byte: 6, end_byte: 10, start_point: GPoint { row: 2, column: 1 }, end_point: GPoint { row: 3, column: 5 } } });
        ast_graph.graph.add_edge(a, b, ());

        // Serialize the ASTGraph to a file for testing
        let serializable_graph = ast_graph.to_serializable();
        let file_path = "test_graph.bin";
        let file = File::create(file_path).expect("Failed to create file for serialization");
        serialize_into(file, &serializable_graph).expect("Serialization error");

        // Deserialization (you can implement this in the ASTGraph struct)
        let file = File::open(file_path).expect("Failed to open file for deserialization");
        let deserialized_graph: SerializableGraph = deserialize_from(file).expect("Deserialization error");

        let reconstructed_graph = ASTGraph::from_serializable(deserialized_graph);
        let number_of_reconstruced_nodes = reconstructed_graph.node_count();
        assert_eq!(number_of_reconstruced_nodes,2);

        let node = reconstructed_graph.get_node( a );
        assert_eq!(node,Some(1));

    }

    #[test]
    fn simple_deserialization_test2()
    {
        // Create a sample ASTGraph for testing
        let mut ast_graph = ASTGraph::new("testing".to_string());
        let a = ast_graph.graph.add_node(GNode { id: 1, kind_id: 1, range: GRange { start_byte: 0, end_byte: 5, start_point: GPoint { row: 1, column: 1 }, end_point: GPoint { row: 1, column: 5 } } });
        let b = ast_graph.graph.add_node(GNode { id: 2, kind_id: 3, range: GRange { start_byte: 6, end_byte: 10, start_point: GPoint { row: 2, column: 1 }, end_point: GPoint { row: 2, column: 5 } } });
        let c = ast_graph.graph.add_node( GNode { id: 4, kind_id: 7, range:  GRange { start_byte: 11, end_byte: 15, start_point: GPoint { row: 3, column: 1 }, end_point: GPoint { row: 3, column: 5 } } });
        ast_graph.graph.add_edge(a, b, ());
        ast_graph.graph.add_edge(a, c, ());
        
        // Serialize the ASTGraph to a file for testing
        let serializable_graph = ast_graph.to_serializable();
        let file_path = "test_graph2.bin";
        let file = File::create(file_path).expect("Failed to create file for serialization");
        serialize_into(file, &serializable_graph).expect("Serialization error");

        // Deserialization (you can implement this in the ASTGraph struct)
        let file = File::open(file_path).expect("Failed to open file for deserialization");
        let deserialized_graph: SerializableGraph = deserialize_from(file).expect("Deserialization error");

        let reconstructed_graph = ASTGraph::from_serializable(deserialized_graph);
        let number_of_reconstruced_nodes = reconstructed_graph.node_count();
        assert_eq!(number_of_reconstruced_nodes,3);

        let node = reconstructed_graph.get_node( a );
        assert_eq!(node,Some(1));

        let number_of_reconstructed_edges = reconstructed_graph.graph.edge_count();
        assert_eq!(number_of_reconstructed_edges,2);

        let endpoint = reconstructed_graph.get_node( c );
        assert_eq!(endpoint,Some(4));

    }

    #[test] 
    fn test_source_splitting() 
    {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_fortran::language()).expect("Error loading Fortran grammar");

        let tree = parser.parse(FORTRAN_CODE, None).unwrap();

        let mut ast_graph = ASTGraph::new(FORTRAN_CODE.to_string());
        ast_graph.build_from_tree(&tree);

        // Extract subgraphs for each function
        let fortran_subroutine_kind:u16 = 229;
        let fortran_program_kind:u16 = 212;
        let mut fortran_types_to_split = HashSet::new();

        fortran_types_to_split.insert(fortran_subroutine_kind);
        fortran_types_to_split.insert(fortran_program_kind);

        let total_source = ast_graph.source.len();
        let mut subgraph_counts = Vec::new();

        let function_subgraphs = ast_graph.extract_subgraphs( fortran_types_to_split );
        for subgraph in function_subgraphs.iter() {
            let count = subgraph.source.len();
            subgraph_counts.push(count);
        }

        let subgraph_totals:usize = subgraph_counts.iter().sum();

        assert!(subgraph_totals < total_source); // due to comments, etc, that are chopped out of the splitting.
    }

    const CPP_STRING:&str = r#"
    #include <iostream>
    #include <fstream>
    #include <string>

    void readFile(const std::string& filePath) {
        std::ifstream file(filePath);

        if (!file.is_open()) {
            std::cerr << "Error reading file: Could not open the file." << std::endl;
            return;
        }

        std::string line;
        while (std::getline(file, line)) {
            std::cout << line << std::endl;
        }

        file.close();
    }

    int main() {
        std::string filePath;

        // Prompt the user for the file path
        std::cout << "Enter the file path: ";
        std::getline(std::cin, filePath);

        readFile(filePath);

        return 0;
    }
    "#;

    #[test]
    fn correct_split_for_cpp() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::language()).expect("Error loading CPP grammar");

        let tree = parser.parse(CPP_STRING, None).unwrap();
        let _root = tree.root_node();

        let mut ast_graph = ASTGraph::new(CPP_STRING.to_string());
        ast_graph.build_from_tree(&tree);

        // Extract subgraphs for each function
        let cpp_function_definition:u16 = 250;
        let mut cpp_types_to_split = HashSet::new();

        cpp_types_to_split.insert(cpp_function_definition);

        let function_subgraphs = ast_graph.extract_subgraphs( cpp_types_to_split );
        // we should get 2 subroutines and 1 program = 3 subgraphs
        assert_eq!(function_subgraphs.len(),2);
    }

    // iterator tests
    #[test]
    fn test_bfs_iterator() {
        let mut ast_graph = ASTGraph::new("testing".to_string());
        let a = ast_graph.graph.add_node(GNode { id: 1, kind_id: 1, range: GRange { start_byte: 0, end_byte: 5, start_point: GPoint { row: 1, column: 1 }, end_point: GPoint { row: 1, column: 5 } } });
        let b = ast_graph.graph.add_node(GNode { id: 2, kind_id: 3, range: GRange { start_byte: 6, end_byte: 10, start_point: GPoint { row: 2, column: 1 }, end_point: GPoint { row: 2, column: 5 } } });
        let c = ast_graph.graph.add_node( GNode { id: 4, kind_id: 7, range:  GRange { start_byte: 11, end_byte: 15, start_point: GPoint { row: 3, column: 1 }, end_point: GPoint { row: 3, column: 5 } } });
        ast_graph.graph.add_edge(a, b, ());
        ast_graph.graph.add_edge(a, c, ());

        let mut nodes_touched = 0;

        let mut bfs_traversal = ast_graph.bfs_iterator(a);
    
        while let Some(_node_index) = bfs_traversal.next(&ast_graph.graph) {
            // Process each visited node
            nodes_touched += 1;
        }

        assert_eq!(nodes_touched,3);
    }

    #[test]
    fn test_dfs_iterator() {
        let mut ast_graph = ASTGraph::new("testing".to_string());
        let a = ast_graph.graph.add_node(GNode { id: 1, kind_id: 1, range: GRange { start_byte: 0, end_byte: 5, start_point: GPoint { row: 1, column: 1 }, end_point: GPoint { row: 1, column: 5 } } });
        let b = ast_graph.graph.add_node(GNode { id: 2, kind_id: 3, range: GRange { start_byte: 6, end_byte: 10, start_point: GPoint { row: 2, column: 1 }, end_point: GPoint { row: 2, column: 5 } } });
        let c = ast_graph.graph.add_node( GNode { id: 4, kind_id: 7, range:  GRange { start_byte: 11, end_byte: 15, start_point: GPoint { row: 3, column: 1 }, end_point: GPoint { row: 3, column: 5 } } });
        ast_graph.graph.add_edge(a, b, ());
        ast_graph.graph.add_edge(a, c, ());

        let mut nodes_touched = 0;

        let mut dfs_traversal = ast_graph.dfs_iterator(a);
    
        while let Some(_node_index) = dfs_traversal.next(&ast_graph.graph) {
            // Process each visited node
            nodes_touched += 1;
        }

        assert_eq!(nodes_touched,3);
    }

    const CPP_STRING_TRIMMED:&str = r#"
    void readFile(const std::string& filePath) {
        std::ifstream file(filePath);

        if (!file.is_open()) {
            std::cerr << "Error reading file: Could not open the file." << std::endl;
            return;
        }

        std::string line;
        while (std::getline(file, line)) {
            std::cout << line << std::endl;
        }

        file.close();
    }
    int main() {
        std::string filePath;

        // Prompt the user for the file path
        std::cout << "Enter the file path: ";
        std::getline(std::cin, filePath);

        readFile(filePath);

        return 0;
    }
    "#;

    #[test]
    fn test_split_cpp_source() 
    {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::language()).expect("Error loading CPP grammar");

        let tree = parser.parse(CPP_STRING_TRIMMED, None).unwrap();

        let mut ast_graph = ASTGraph::new(CPP_STRING_TRIMMED.to_string());
        ast_graph.build_from_tree(&tree);

        // Extract subgraphs for each function
        let cpp_function_definition:u16 = 250;
        let mut cpp_types_to_split = HashSet::new();

        cpp_types_to_split.insert(cpp_function_definition);

        let function_subgraphs = ast_graph.extract_subgraphs( cpp_types_to_split );
        // we should get 2 subroutines and 1 program = 3 subgraphs
        assert_eq!(function_subgraphs.len(),2);

        // check the source lengths
        let total_length = ast_graph.source.len();
        let mut function_total_length = 0;

        for func in function_subgraphs {
            function_total_length += func.source.len();
        }

        // 15 bytes extra, even in the trimmed case.
        assert_eq!(function_total_length + 15, total_length);

    }

    #[test]
    fn save_cpp_subgraphs() 
    {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::language()).expect("Error loading CPP grammar");

        let tree = parser.parse(CPP_STRING_TRIMMED, None).unwrap();

        let mut ast_graph = ASTGraph::new(CPP_STRING_TRIMMED.to_string());
        ast_graph.build_from_tree(&tree);

        // Extract subgraphs for each function
        let cpp_function_definition:u16 = 250;
        let mut cpp_types_to_split = HashSet::new();

        cpp_types_to_split.insert(cpp_function_definition);

        let function_subgraphs = ast_graph.extract_subgraphs( cpp_types_to_split );

        assert_eq!(function_subgraphs.len(),2);

        for subgraph in function_subgraphs {
            let name = subgraph.name();
            let filename = format!("{}.bin",name);
            let file = File::create(filename.clone() ).expect("Failed to open subgraph file.");
            let serializaable_graph = subgraph.to_serializable();
            let result = serialize_into(file, &serializaable_graph);

            let file_exists = check_file_existence_and_nonempty(filename.as_str() );
            match file_exists {
                Ok(true) => {
                    assert!(true);
                },
                Ok(false) => {
                    assert!(false);
                }
                Err(_err) => {
                    assert!(false);
                }
            }
        }
        
    }

    // utility function to get the node count.
    fn count_nodes(cursor: &mut TreeCursor) -> usize {
        let mut count = 0;
    
        count += 1; // Count the current node
    
        if cursor.goto_first_child() {
            count += count_nodes(cursor); // Recursively count children nodes
            while cursor.goto_next_sibling() {
                count += count_nodes(cursor); // Recursively count sibling nodes
            }
            cursor.goto_parent();
        }
    
        count
    }

    #[test]
    fn tree_sitter_node_count_test() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_cpp::language()).expect("Error loading CPP grammar");

        let tree = parser.parse(CPP_STRING_TRIMMED, None).unwrap();

        let mut ast_graph = ASTGraph::new(CPP_STRING_TRIMMED.to_string());
        ast_graph.build_from_tree(&tree);

        let mut cursor = tree.root_node().walk();
        let tree_node_count = count_nodes(&mut cursor);
        let ast_node_count  = ast_graph.node_count();

        assert_eq!(tree_node_count, ast_node_count);
    }

    #[test]
    fn test_simple_path() {

        let mut ast_graph = ASTGraph::new("testing".to_string());
        let a = ast_graph.graph.add_node(GNode { id: 1, kind_id: 1, range: GRange { start_byte: 0, end_byte: 5, start_point: GPoint { row: 1, column: 1 }, end_point: GPoint { row: 1, column: 5 } } });
        let b = ast_graph.graph.add_node(GNode { id: 2, kind_id: 3, range: GRange { start_byte: 6, end_byte: 10, start_point: GPoint { row: 2, column: 1 }, end_point: GPoint { row: 2, column: 5 } } });
        let c = ast_graph.graph.add_node( GNode { id: 4, kind_id: 72, range:  GRange { start_byte: 11, end_byte: 15, start_point: GPoint { row: 3, column: 1 }, end_point: GPoint { row: 3, column: 5 } } });
        let d = ast_graph.graph.add_node( GNode { id: 5, kind_id: 37, range:  GRange { start_byte: 16, end_byte: 20, start_point: GPoint { row: 4, column: 1 }, end_point: GPoint { row: 4, column: 5 } } });
        let e = ast_graph.graph.add_node( GNode { id: 7, kind_id: 4, range:  GRange { start_byte: 21, end_byte: 25, start_point: GPoint { row: 5, column: 1 }, end_point: GPoint { row: 5, column: 5 } } });
        let f = ast_graph.graph.add_node( GNode { id: 10, kind_id: 7, range:  GRange { start_byte: 26, end_byte: 30, start_point: GPoint { row: 6, column: 1 }, end_point: GPoint { row: 6, column: 5 } } });

        ast_graph.graph.add_edge(a, b, ());
        ast_graph.graph.add_edge(a, c, ());
        ast_graph.graph.add_edge(c, d, ());
        ast_graph.graph.add_edge(c, e, ());
        ast_graph.graph.add_edge(d, f, ());

        let optional_path = ast_graph.path_from_to(a, f);
        let true_path = vec![a,c,d,f];

        if let Some(path) = optional_path {
            for (i,j) in path.iter().zip( true_path.iter()) {
                assert_eq!(i,j);
            }
        } else {
            assert!(false);
        }
    
    }

}
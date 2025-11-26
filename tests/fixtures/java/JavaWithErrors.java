package com.example.bugs;

import java.util.List;

/**
 * Class with various Java errors for testing diagnostics.
 */
public class JavaWithErrors {
    private String uninitialized;
    
    public void methodWithErrors() {
        // Uninitialized variable usage
        System.out.println(uninitialized.length());
        
        // Potential null pointer
        String potentiallyNull = null;
        System.out.println(potentiallyNull.length());
        
        // Unused variable
        int unused = 42;
        
        // Missing return statement
        if (true) {
            System.out.println("Always true");
        }
    }
    
    public void anotherMethod(List<String> items) {
        // Resource leak - not closing resources properly
        try {
            for (String item : items) {
                System.out.println(item.toUpperCase());
            }
        } catch (Exception e) {
            // Empty catch block
        }
    }
}
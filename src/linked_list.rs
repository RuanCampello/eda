#![allow(unused)]

struct LinkedList<T> {
    head: Link<T>,
}

struct Node<T> {
    element: T,
    next: Link<T>,
}

// em rust, não é possivel fazer uma estrutura que se alto referencia sem um "indicador"
// de onde isso fica na memória. Como uma linked list é um desses casos, precisamos indicar que
// os nodes ficam na heap com o ponteiro Box.
type Link<T> = Option<Box<Node<T>>>;

impl<T> LinkedList<T> {
    fn new() -> Self {
        Self { head: None }
    }

    fn push(&mut self, element: T) {
        // esse é um dos tipos de codigos que me faz usar rust.
        // é extremamente simples e faz EXATAMENTE o que foi descrito:
        // - cria um novo ponteiro na heap
        // - cria um node novo com o elemento passado
        // - MOVEMOS o ponteiro da head para esse novo node com `take()`, que toma a
        // propriedade do antigo ponteiro
        // - movemos a head para o novo node
        let new_node = Box::new(Node {
            element,
            next: self.head.take(),
        });

        self.head = Some(new_node);
    }

    fn pop(&mut self) -> Option<T> {
        self.head.take().map(|node| {
            self.head = node.next;
            node.element
        })
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();

        // basicamente iteramos sobre todos nodes chamando .next enquanto
        // existir
        while let Some(mut node) = current {
            current = node.next.take()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_pop() {
        let mut list = LinkedList::new();
        assert_eq!(list.pop(), None); // ainda não adicionamos nada ent deveria estar vazio

        list.push(1);
        list.push(2);
        list.push(3);

        assert_eq!(list.pop(), Some(3));
        assert_eq!(list.pop(), Some(2));

        list.push(4);
        list.push(5);

        assert_eq!(list.pop(), Some(5));
        assert_eq!(list.pop(), Some(4));

        assert_eq!(list.pop(), Some(1));
        assert_eq!(list.pop(), None);
    }
}

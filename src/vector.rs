#![allow(unused)]

use std::{
    alloc::{self, Layout},
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

/// Vetor (lista dinâmica alocada na HEAP)
struct Vector<T> {
    // ponteiro que aponta para o conteúdo da célula.
    // em rust, usamos `NonNull` para indicar pro compilador que esse ponteiro
    // nunca deve ser nulo.
    // isso também abre portas para mais otimizações por parte do compilador.
    ptr: NonNull<T>,
    capacity: usize,
    length: usize,
}

// rust é paranoico com threads. ponteiros (*mut T) não implementam send/sync automaticamente
// porque o compilador não sabe se é seguro. estamos basicamente dizendo "confia no pai".
unsafe impl<T: Send> Send for Vector<T> {}
unsafe impl<T: Sync> Sync for Vector<T> {}

impl<T> Vector<T> {
    fn new() -> Self {
        // precisamos fazer esse assert porque rust tem lida diferente com valores de tamanho zero.
        assert!(
            std::mem::size_of::<T>() != 0,
            "Zero-sized type are not supported >:X"
        );

        Self {
            // `dangling` cria um ponteiro não-nulo invalido mas alinhado.
            // o que é seguro, a não ser que a gente o deferencie.
            ptr: NonNull::dangling(),
            capacity: 0,
            length: 0,
        }
    }

    const fn len(&self) -> usize {
        self.length
    }

    const fn capacity(&self) -> usize {
        self.capacity
    }

    fn push(&mut self, element: T) {
        if self.length == self.capacity {
            self.grow();
        }

        unsafe {
            let end = self.ptr.as_ptr().add(self.length);

            // em rust, não podemos fazer *end = element.
            // ele tentaria executar o `drop` no lixo que está na memória
            // antiga antes de sobreescrever.
            // ptr::write é parecido com memcpy.
            ptr::write(end, element);
        }

        self.length += 1;
    }

    fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            return None;
        }

        self.length -= 1;

        unsafe {
            let end = self.ptr.as_ptr().add(self.length);
            // ptr::read copia os bits para fora da memória e transfere a propriedade pro destino.
            Some(ptr::read(end))
        }
    }

    fn grow(&mut self) {
        // malloc / realloc em rust exigem alinhamento explícito.
        // o layout guarda o size + alignment. se errarmos o alinhamento, é undefined behaviour
        // (terra do diabo). considere parecido com posix_memalign em vez do malloc
        let (new_capacity, new_layout) = match self.capacity == 0 {
            true => (1, Layout::array::<T>(1).unwrap()),
            false => {
                let new_capacity = self.capacity * 2;
                let new_layout = Layout::array::<T>(new_capacity).unwrap();
                (new_capacity, new_layout)
            }
        };

        assert!(
            new_layout.size() <= isize::MAX as usize,
            "Allocation too large"
        );

        let new_ptr = match self.capacity == 0 {
            true => unsafe { alloc::alloc(new_layout) },
            false => {
                let old_layout = Layout::array::<T>(self.capacity).unwrap();
                let old_ptr = self.ptr.as_ptr() as *mut u8;
                unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
            }
        };

        // tratamento simples para out of memory
        self.ptr = NonNull::new(new_ptr as *mut T).expect("Memory allocation failed");
        self.capacity = new_capacity
    }
}

// implementar deref faz o papel do `decay` em c++.
// permite tratar &Vector como &[T] (slice).
// o slice em rust é um fat pointer (ponteiro + tamanho) nativo da linguagem.
impl<T> Deref for Vector<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.length) }
    }
}

impl<T> DerefMut for Vector<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.length) }
    }
}

// como vector é uma estrutura que é alocada na heap e fazemos isso manualmente
// ao implementarmos drop, ao sair do escopo da função, a linguagem saberá como liberar essa
// estrutura da memória corretamente.
impl<T> Drop for Vector<T> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            // iteramos todos os items e os removemos da memória com o pop
            // que usa o ptr::read
            while let Some(_) = self.pop() {}

            let layout = Layout::array::<T>(self.capacity).unwrap();
            unsafe { alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop_basic() {
        let mut v = Vector::new();
        v.push(1);
        v.push(2);
        v.push(3);

        assert_eq!(v.len(), 3);

        // verifica ordem lifo (last in, first out) no pop
        assert_eq!(v.pop(), Some(3));
        assert_eq!(v.pop(), Some(2));
        assert_eq!(v.len(), 1);

        v.push(4);
        assert_eq!(v.pop(), Some(4));
        assert_eq!(v.pop(), Some(1));
        assert_eq!(v.pop(), None); // underflow check
    }

    #[test]
    fn test_growth() {
        let mut vec = Vector::new();

        for n in 0..10 {
            vec.push(n);
        }

        assert_eq!(vec.len(), 10);
        assert!(vec.capacity() >= 10);

        // isso testa nossa implementação de deref / deref_mut :D
        assert_eq!(vec[0], 0);
        assert_eq!(vec[5], 5);
    }

    #[test]
    fn test_iterators() {
        // uma das grandes magias de rust são seus iteradores (fortemente inspirado em linguagens
        // funcionais, como ocaml)
        // então, como implementamos deref que faz nosso vetor ser tratado como um slice,
        // deveriamos ser agraciados com essa magia tbm >:)
        //

        let mut vec = Vector::new();
        vec.push("Hello");
        vec.push("World");

        // basicamente podemos transformar nossa implementação em um iterator nativo da linguagem
        // (isso é mt lindo)
        // e então usarmos ferramentas da linguagem e coletar isso como um vetor padrão de rust.
        let lengths: Vec<usize> = vec.iter().map(|string| string.len()).collect();
        assert_eq!(lengths, vec![5, 5])
    }

    #[test]
    fn test_complex_types_strings() {
        let mut v = Vector::new();
        // força múltiplos reallocs com strings (que têm sua própria heap)
        for i in 0..20 {
            v.push(format!("String {}", i));
        }

        assert_eq!(v.len(), 20);
        assert_eq!(v[0], "String 0");
        assert_eq!(v[19], "String 19");

        // se o realloc fosse feito errado (ex: shallow copy sem cuidado),
        // ao acessar essas strings teríamos segfault (double free ou use-after-free).
    }
}

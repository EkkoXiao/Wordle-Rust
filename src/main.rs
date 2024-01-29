use std::collections::{BTreeSet, HashMap, BinaryHeap, HashSet};
use std::cmp::Ordering;
use clap::{Parser, ArgGroup};
use rand::seq::SliceRandom;
use rand::rngs::StdRng;
use rand::SeedableRng;
use text_io::read;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use serde_json::{Map, Value};
use std::fs;
use crate::builtin_words::{FINAL, ACCEPTABLE};
mod builtin_words;
use console;
use std::io::{self, Write};

#[derive(Parser, Deserialize, Serialize)]
#[clap(author, version, about, long_about = None)]
#[clap(group(
            ArgGroup::new("mode")
            .required(false)
            .args(&["word","random"]),
))]
#[clap(group(
            ArgGroup::new("rand_day")
            .requires("random")
            .conflicts_with("word")
            .args(&["day"]),
))]
#[clap(group(
            ArgGroup::new("rand_seed")
            .requires("random")
            .conflicts_with("word")
            .args(&["seed"]),
))]
///Arguments
struct Args {
    #[clap(short, long, value_parser)]
    word: Option<String>,
    #[clap(short, long, action)]
    random: bool,
    #[clap(short, long, default_value_t = 1, value_parser)]
    day: usize,
    #[clap(short, long, default_value_t = 114514, value_parser)]
    seed: u64,
    #[clap(short = 'D', long, action)]
    difficult: bool,
    #[clap(short = 't', long, action)]
    stats: bool,
    #[clap(short = 'f', long = "final-set", value_parser)]
    finalset: Option<String>,
    #[clap(short = 'a', long = "acceptable-set", value_parser)]
    acceptableset: Option<String>,
    #[clap(short = 'S', long, value_parser)]
    state: Option<String>,
    #[clap(short = 'c', long, value_parser)]
    config: Option<String>,
    #[clap(short, long, value_parser)]
    idea: Option<bool>
}

///Tuple for one word(string) and its appearence(i32)
#[derive(Debug, Eq, Ord)]
struct WordDict (String, i32);

impl PartialEq for WordDict {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for WordDict {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.1.cmp(&other.1) {
            Ordering::Greater => Some(Ordering::Greater),
            Ordering::Less => Some(Ordering::Less),
            Ordering::Equal => Some(self.0.cmp(&other.0).reverse())
        }
    }
}

///Tuple for one word(string) and its information entrophy
#[derive(Debug, Eq, Ord)]
struct WordEntrophy (String, i64);

impl PartialEq for WordEntrophy {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for WordEntrophy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.1.cmp(&other.1) {
            Ordering::Greater => Some(Ordering::Greater),
            Ordering::Less => Some(Ordering::Less),
            Ordering::Equal => Some(self.0.cmp(&other.0).reverse())
        }
    }
}

///Indicating the result of one game round
struct GameResult {
    win: bool,
    attempt: i32,
    word_list: HashMap<String, i32>
}

///Answer and guesses for one game round
#[derive(Deserialize, Serialize, Debug)]
struct Game {
    answer: String,
    guesses: Vec<String>
}

///All games used to load into json
#[derive(Deserialize, Serialize, Debug)]
struct User {
    total_rounds: Option<i32>,
    games: Option<Vec<Game>>
}

///Compute each words' information entrophy in the library
fn information_entrophy(library: &BTreeSet<String>) -> BinaryHeap<WordEntrophy> {
    let mut result: BinaryHeap<WordEntrophy> = BinaryHeap::new();
    let mut state = [[0; 5]; 243];
    for i in 1..243 {
        let mut num = i;
        state[i][0] = num / 81;
        num %= 81;
        state[i][1] = num / 27;
        num %= 27;
        state[i][2] = num / 9;
        num %= 9;
        state[i][3] = num / 3;
        num %= 3;
        state[i][4] = num;
    }//生成状态数组共3^5种
    //2->Green 1->Yellow 0->Red
    
    for word_target in library {
        let mut index = 0;
        let mut flag_list: HashSet<String> = HashSet::new();
        let mut condition = [0; 243];
        for state in &state {
            for word in library {
                if flag_list.contains(word) { continue; }
                //新建target字符集映射，反映各字母出现次数
                let mut target_map: HashMap<char, i32> = HashMap::new();
                for c in word_target.chars() {
                    if let Some(x) = target_map.get_mut(&c) {
                    *x += 1;
                    }
                    target_map.entry(c).or_insert(1);
                }
                let mut map: HashMap<char, i32> = HashMap::new();
                for c in word.chars() {
                    if let Some(x) = map.get_mut(&c) {
                    *x += 1;
                    }
                    map.entry(c).or_insert(1);
                }
                let mut flag = true;
                //Green
                for i in 0..5 {
                    if state[i] == 2 && word.chars().nth(i) != word_target.chars().nth(i) {
                        flag = false;
                        break;
                    }
                    else if state[i] == 2 && word.chars().nth(i) == word_target.chars().nth(i) {
                        if let Some(x) = word.chars().nth(i) {
                            if let Some(z) = target_map.get_mut(&x) {
                                *z -= 1;
                            }
                            if let Some(z) = map.get_mut(&x) {
                                *z -= 1;
                            }
                        }
                    }
                }
                if !flag { continue; }
                //Yellow
                for i in 0..5 {
                    if state[i] == 1 {
                        if word.chars().nth(i) == word_target.chars().nth(i) { flag = false; break; }
                        else if let Some(x) = word_target.chars().nth(i) {
                            if let Some(z) = map.get_mut(&x) {
                                if *z == 0 { flag = false; break; }
                                else { *z -= 1; }
                            }
                            else { flag = false; break; }
                        }
                    }
                }
                if !flag { continue; }
                //Red
                for i in 0..5 {
                    if state[i] == 0 {
                        if let Some(x) = word_target.chars().nth(i) {
                            if let Some(z) = map.get_mut(&x) {
                                if *z != 0 { flag = false; break; }
                            }
                        }
                    }
                }
                if !flag { continue; }
                if flag {
                    condition[index] += 1;
                    flag_list.insert(word.clone());
                }
            }
            index += 1;
        }       
        let mut sum: f64 = 0.0;
        for i in condition {
            let p = i as f64 / library.len() as f64;
            if i != 0 && i != 1 {
                sum += p * (1.0 / p).log2();
            }
        }
        //println!("{} {}",word_target, sum);
        result.push(WordEntrophy(word_target.clone(), (sum * 1000000000.0) as i64));
    }
    result
}

///Spawn required index in FINAL list
fn random_spawn(day: usize, seed: u64, size: usize) -> usize{
    let len = size;
    let mut list: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < len {
        list.push(i);
        i += 1;
    }
    let mut r = StdRng::seed_from_u64(seed);
    list.shuffle(&mut r);
    list[day - 1]
}

///Read txt files and load into vector
fn read_to_list(path: String) -> BTreeSet<String> {
    let model_filename: String = std::fs::read_to_string(path).unwrap();

    let result: BTreeSet<String> = model_filename
        .lines()
        .into_iter()
        .map(move |ch| ch.to_string().to_lowercase())
        .collect();
    let mut returnvalue: BTreeSet<String> = BTreeSet::new();
    for x in &result {
        let mut flag = true;
        for c in x.chars() {
            if !c.is_alphabetic() || x.len() != 5 {
                flag = false;
                break;
            }
        }
        if flag {
            returnvalue.insert(x.clone());
        }
        else {
            panic!("Invalid file assigned");
        }
    }
    returnvalue
}

///Read json files and load into User
fn read_from_file_user(path: &String) -> User{
    let data = fs::read_to_string(path).unwrap();

    let u: User = serde_json::from_str(&data).unwrap();
    u
}

///Read json files and load into Args
fn read_from_file_config(path: &String) -> Args{
    let data = fs::read_to_string(path).unwrap();

    let u: Args = serde_json::from_str(&data).unwrap();
    u
}

///Normal state for one wordle round
fn game_round_normal(args: &Args) -> Option<GameResult> {
    let mut final_words: BTreeSet<String> = BTreeSet::new();
    let mut acceptable_words: BTreeSet<String> = BTreeSet::new();
    let mut final_flag = false;
    let mut acceptable_flag = false;
    let mut entrophy: BinaryHeap<WordEntrophy> = BinaryHeap::new();
    //导入文件词库
    if let Some(path) = &args.finalset {
        final_words = read_to_list(path.clone());
        final_flag = true;
    }
    else {
        for word in FINAL {
            final_words.insert(word.to_string());
        }
    }
    if let Some(path) = &args.acceptableset {
        acceptable_words = read_to_list(path.clone());
        acceptable_flag = true;
        entrophy = information_entrophy(&acceptable_words);
    }
    else {
        for word in ACCEPTABLE {
            acceptable_words.insert(word.to_string());
        }
        let config = fs::read_to_string("src/acceptable.json").unwrap();
        let parsed: Value = serde_json::from_str(&config).unwrap();
        let obj: Map<String, Value> = parsed.as_object().unwrap().clone();
        for i in obj {
            let x: i64 = serde_json::from_value(i.1).unwrap();
            entrophy.push(WordEntrophy(i.0.clone(), x));
        }
    }
    let mut reasonable_words = acceptable_words.clone();//指示针对猜测是否是合法单词集
    //检查是否是子集
    if final_flag || acceptable_flag {
        let mut flag = true;
        for word in &final_words {
            if !acceptable_words.contains(word) {
                flag = false;
                break;
            }
        }
        if !flag { panic!("Invalid word set--not included."); }
    }

    let mut return_value: Option<GameResult>= None;
    if args.stats {
        return_value = Some(GameResult{ win:false, attempt:0, word_list: HashMap::new()});
    }
    if args.difficult {
        println!("{}! You choosed {} mode!",console::style("Warning").bold().red(),console::style("DIFFICULT").bold().red())
    }
    let mut answer;
    if args.random {//随机模式启动
        let index = random_spawn(args.day, args.seed, final_words.len());
        let mut final_words_vec: Vec<&String> = Vec::new();
        for word in &final_words {
            final_words_vec.push(word);
        }
        answer = final_words_vec[index].to_string();
    }
    else if let Some(ans) = &args.word {//答案已指定
        answer = ans.to_lowercase().trim().to_string();
    }
    else {
        //输入answer
        loop {
            print!("Please input the answer here: ");
            answer = String::new();
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut answer).unwrap();
            answer = answer.to_lowercase().trim().to_string();
            if final_words.contains(&answer) { break; }
            else { println!("Invalid answer! Please try again!"); }
        }
    }
    println!("Answer assigned: {} :)", answer.to_uppercase());

    let mut u: User = User { total_rounds: None, games: None };//Json文件的结构体格式
    let mut game = Game{answer: answer.to_uppercase().clone(), guesses: Vec::new()};
    if let Some(path) = &args.state {//需要加载状态Json文件
        u = read_from_file_user(path);
        if let Some(ref mut x ) = u.total_rounds {
            *x += 1;
        }
        else {
            u.total_rounds = Some(1);
        }
        if let None = u.games {
            u.games = Some(Vec::new());
        }
    }
    //字母表映射各字符状态
    let mut alphabet: HashMap<char, char> = HashMap::new();
    for c in 'a'..'{' {
        alphabet.insert(c, 'X');
    }
    //困难模式使用的判断
    let mut yellow_letters: Vec<char> = Vec::new();
    let mut green_letters= ['0';5];

    let mut chances = 1;
    while chances <=6 {
        //新建answer字符集映射
        let mut answer_map: HashMap<char, i32> = HashMap::new();
        for c in answer.chars() {
            if let Some(x) = answer_map.get_mut(&c) {
                *x += 1;
            }
            answer_map.entry(c).or_insert(1);
        }
        //输入guess
        //信息熵提示
        let mut i = 0;
        print!("Do you want any suggestions? {}/{} ",
                        console::style("[Y]").bold().yellow(),
                        console::style("[N]").bold().red());
        let command: char = read!();
        if command == 'Y' {
            println!("Possible guesses: {} in total", console::style(reasonable_words.len()).bold().yellow());
            println!("{} and their {}:",
                            console::style("Most possible words").bold().green(),
                            console::style("Information Entrophy").bold().red());
            while !entrophy.is_empty() && i < 5 {
                if let Some(word) = entrophy.pop(){
                    println!("{} {:.4}",
                        console::style(&word.0.to_uppercase()).bold().green(),
                        console::style(word.1 as f64 / 1000000000.0).bold().red());
                }
                i += 1;
            }
        }
        print!("Please input your guess here: ");
        io::stdout().flush().unwrap();
        let mut guess = String::new();
        io::stdin().read_line(&mut guess).unwrap();
        guess = guess.to_lowercase().trim().to_string();
        let guess_str = &guess[..];
        //不合法判断处理
        if guess.len() != 5 || !ACCEPTABLE.contains(&guess_str) {
            println!("Invalid input! :( Please guess again!");
            continue;
        }
        else {
            let mut flag = true;
            for c in guess.chars() {
                if !c.is_alphabetic() {
                    println!("Invalid input! :( Please guess again!");
                    flag = false;
                    break;
                }
            }
            if !flag {
                continue;
            }
        }
        //合法情况
        
        if guess == answer {//直接猜出答案
            println!("{}",console::style(answer.to_uppercase()).green());
            println!("Correct! :D You tried {} times.", chances);
            if args.stats {
                if let Some(ref mut x) = return_value {
                    x.win = true;
                    x.attempt = chances;
                    let count = x.word_list.entry(guess.clone()).or_insert(0);
                    *count+=1;
                }
            }
            game.guesses.push(guess.to_uppercase().clone());
            if let Some(ref mut x) = u.games {
                x.push(game);
            }
            let result = to_string_pretty(&u).unwrap();
            if let Some(path) = &args.state{
                fs::write(path, result).unwrap();
            }
            return return_value;
        }
        let mut show = ['X'; 5];
        if !args.difficult {//非困难模式
            if args.stats {
                if let Some(ref mut x) = return_value {
                    let count = x.word_list.entry(guess.clone()).or_insert(0);
                    *count+=1;
                }
            }
            game.guesses.push(guess.to_uppercase().clone());
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                        if let Some(y) = answer.chars().nth(i) {
                            if x == y {
                                show[i] = 'G';
                                if let Some(z) = answer_map.get_mut(&x) {
                                    *z -= 1;
                                }
                                if let Some(z) = alphabet.get_mut(&x) {
                                    *z = 'G';
                                }
                            }
                        }
                    }
                }
            for i in 0..5 {
            if let Some(x) = guess.chars().nth(i) {
                    if show[i] != 'G' {
                        if answer_map.get(&x) == Some(&0) || answer_map.get(&x) == None {
                            show[i] = 'R';
                            if let Some(z) = alphabet.get_mut(&x) {
                                if *z == 'X' {
                                    *z = 'R'; 
                                }
                            }
                        }
                        else {
                            show[i] = 'Y';
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                            if let Some(z) = alphabet.get_mut(&x) {
                                if *z != 'G' {
                                    *z = 'Y'; 
                                }
                            }
                        }
                    }
                }
            }
        }
        //困难模式
        else {
            let mut flag =true;
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if green_letters[i] != '0' && green_letters[i] != x {
                        println!("Invalid input for {} letters not int place! :( Please guess again!", console::style("GREEN").green());
                        flag = false;
                        break;
                    }
                }
            }
            for i in yellow_letters.clone() {
                if !guess.contains(i) {
                        println!("Invalid input for {} letters not used! :( Please guess again!", console::style("YELLOW").yellow());
                        flag = false;
                        break;
                }
            }
            if !flag {
                continue;
            }
            if args.stats {
                if let Some(ref mut x) = return_value {
                    let count = x.word_list.entry(guess.clone()).or_insert(0);
                    *count+=1;
                }
            }
            game.guesses.push(guess.to_uppercase().clone());
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if let Some(y) = answer.chars().nth(i) {
                        if x == y {//绿色
                            show[i] = 'G';
                            green_letters[i] = x;
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                            if let Some(z) = alphabet.get_mut(&x) {
                                *z = 'G';
                            }
                        }
                    }
                }
            }
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if show[i] != 'G' {
                        if answer_map.get(&x) == Some(&0) || answer_map.get(&x) == None {//红色
                            show[i] = 'R'; 
                            if let Some(z) = alphabet.get_mut(&x) {
                                if *z == 'X' {
                                    *z = 'R'; 
                                }
                            }
                        }
                        else {//黄色
                            show[i] = 'Y';
                            yellow_letters.push(x);
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                            if let Some(z) = alphabet.get_mut(&x) {
                                if *z != 'G' {
                                    *z = 'Y'; 
                                }
                            }
                        }
                    }
                }
            }
        }
        //颜色输出结果
        let mut state = [0; 5];
        for i in 0..5 {
            let color = show[i];
            if let Some(letter) = guess.chars().nth(i) {
                match color {
                    'G' => { print!("{}", console::style(letter.to_uppercase()).green()); state[i] = 2; },
                    'Y' => { print!("{}", console::style(letter.to_uppercase()).yellow()); state[i] = 1; },
                    'R' => { print!("{}", console::style(letter.to_uppercase()).red()); state[i] = 0; },
                     _ => unimplemented!()
                }
            }  
        }
        println!("");
        for letter in ['q','w','e','r','t','y','u','i','o','p'] {
            if let Some(color) = alphabet.get_mut(&letter) {
                match color {
                    'G' => { print!("{}", console::style(letter.to_uppercase()).green()); },
                    'Y' => { print!("{}", console::style(letter.to_uppercase()).yellow()); },
                    'R' => { print!("{}", console::style(letter.to_uppercase()).red()); },
                    'X' => { print!("{}", console::style(letter.to_uppercase()).dim()); },
                     _ => unimplemented!()
                }
            }
        }
        println!("");
        print!(" ");
        for letter in ['a','s','d','f','g','h','j','k','l'] {
            if let Some(color) = alphabet.get_mut(&letter) {
                match color {
                    'G' => { print!("{}", console::style(letter.to_uppercase()).green()); },
                    'Y' => { print!("{}", console::style(letter.to_uppercase()).yellow()); },
                    'R' => { print!("{}", console::style(letter.to_uppercase()).red()); },
                    'X' => { print!("{}", console::style(letter.to_uppercase()).dim()); },
                     _ => unimplemented!()
                }
            }
        }
        println!("");
        print!("  ");
        for letter in ['z','x','c','v','b','n','m'] {
            if let Some(color) = alphabet.get_mut(&letter) {
                match color {
                    'G' => { print!("{}", console::style(letter.to_uppercase()).green()); },
                    'Y' => { print!("{}", console::style(letter.to_uppercase()).yellow()); },
                    'R' => { print!("{}", console::style(letter.to_uppercase()).red()); },
                    'X' => { print!("{}", console::style(letter.to_uppercase()).dim()); },
                     _ => unimplemented!()
                }
            }
        }
        //更新信息熵集
        entrophy.clear();
        let reasonable_words_copy = reasonable_words.clone();
        reasonable_words.clear();
        for word in &reasonable_words_copy{
            let mut flag = true;
            let mut target_map: HashMap<char, i32> = HashMap::new();
            for c in guess.chars() {
                if let Some(x) = target_map.get_mut(&c) {
                    *x += 1;
                }
                target_map.entry(c).or_insert(1);
            }
            let mut map: HashMap<char, i32> = HashMap::new();
            for c in word.chars() {
                if let Some(x) = map.get_mut(&c) {
                    *x += 1;
                }
                map.entry(c).or_insert(1);
            }
            //Green
            for i in 0..5 {
                if state[i] == 2 && word.chars().nth(i) != guess.chars().nth(i) {
                    flag = false;
                    break;
                }
                else if state[i] == 2 && word.chars().nth(i) == guess.chars().nth(i) {
                    if let Some(x) = word.chars().nth(i) {
                        if let Some(z) = target_map.get_mut(&x) {
                            *z -= 1;
                        }
                        if let Some(z) = map.get_mut(&x) {
                            *z -= 1;
                        }
                    }
                }
            }
            if !flag { continue; }
            //Yellow
            for i in 0..5 {
                if state[i] == 1 {
                    if word.chars().nth(i) == guess.chars().nth(i) { flag = false; break; }
                    else if let Some(x) = guess.chars().nth(i) {
                        if let Some(z) = map.get_mut(&x) {
                            if *z == 0 { flag = false; break; }
                            else { *z -= 1; }
                        }
                        else { flag = false; break; }
                    }
                }
            }
            if !flag { continue; }
            //Red
            for i in 0..5 {
                if state[i] == 0 {
                    if let Some(x) = guess.chars().nth(i) {
                        if let Some(z) = map.get_mut(&x) {
                            if *z != 0 { flag = false; break; }
                        }
                    }
                }
            }
            if !flag { continue; }
            if flag {
                reasonable_words.insert(word.clone());
            }
        }
        entrophy = information_entrophy(&reasonable_words);
        println!("");
        chances += 1;
    }
    println!("You failed! :( The correct answer is {}", answer.to_uppercase());
    if args.stats {
        if let Some(ref mut x) = return_value {
            x.attempt = chances - 1;
        }
    }
    if let Some(ref mut x) = u.games {
        x.push(game);
    }
    let result = to_string_pretty(&u).unwrap();
    if let Some(path) = &args.state{
        fs::write(path, result).unwrap();
    }
    return return_value;
}

///Test state for one wordle round
fn game_round_test(args: &Args) -> Option<GameResult> {

    let mut final_words: BTreeSet<String> = BTreeSet::new();
    let mut acceptable_words: BTreeSet<String> = BTreeSet::new();
    let mut final_flag = false;
    let mut acceptable_flag = false;
    //导入文件词库
    if let Some(path) = &args.finalset {
        final_words = read_to_list(path.clone());
        final_flag = true;
    }
    else {
        for word in FINAL {
            final_words.insert(word.to_string());
        }
    }
    if let Some(path) = &args.acceptableset {
        acceptable_words = read_to_list(path.clone());
        acceptable_flag = true;
    }
    else {
        for word in ACCEPTABLE {
            acceptable_words.insert(word.to_string());
        }
    }
    //检查是否是子集
    if final_flag || acceptable_flag {
        let mut flag = true;
        for word in &final_words {
            if !acceptable_words.contains(word) {
                flag = false;
                break;
            }
        }
        if !flag { panic!("Invalid word set--not included."); }
    }


    let mut return_value: Option<GameResult>= None;
    if args.stats {
        return_value = Some(GameResult{ win:false, attempt:0, word_list: HashMap::new()});
    }

    let mut answer = String::new();
    if args.random {//随机模式启动
        let index = random_spawn(args.day, args.seed, final_words.len());
        let mut final_words_vec: Vec<&String> = Vec::new();
        for word in &final_words {
            final_words_vec.push(word);
        }
        answer = final_words_vec[index].to_string();
    }
    else if let Some(ans) = &args.word {
        answer = ans.to_lowercase().trim().to_string();
    }
    else {
        //输入answer
        io::stdin().read_line(&mut answer).unwrap();
        answer = answer.to_lowercase().trim().to_string();
    }


    let mut u: User = User { total_rounds: None, games: None };//Json文件的结构体格式
    let mut game = Game{answer: answer.to_uppercase().clone(), guesses: Vec::new()};
    if let Some(path) = &args.state {//需要加载状态Json文件
        u = read_from_file_user(path);
        if let Some(ref mut x ) = u.total_rounds {
            *x += 1;
        }
        else {
            u.total_rounds = Some(1);
        }
        if let None = u.games {
            u.games = Some(Vec::new());
        }
    }    
    //字母表映射各字符状态
    let mut alphabet: HashMap<char, char> = HashMap::new();
    for c in 'a'..'{' {
        alphabet.insert(c, 'X');
    }
    //困难模式使用的判断
    let mut yellow_letters: Vec<char> = Vec::new();
    let mut green_letters= ['0';5];
    //共六次chances
    let mut chances = 1;
    while chances <=6 {
        
        //新建answer字符集映射，反映各字母出现次数
        let mut answer_map: HashMap<char, i32> = HashMap::new();
        for c in answer.chars() {
            if let Some(x) = answer_map.get_mut(&c) {
                *x += 1;
            }
            answer_map.entry(c).or_insert(1);
        }

        //输入guess
        let mut guess = String::new();
        io::stdin().read_line(&mut guess).unwrap();
        guess = guess.to_lowercase().trim().to_string();
        let guess_str = &guess[..];
        //不合法判断处理
        if guess.len() != 5 || !ACCEPTABLE.contains(&guess_str) {//长度不合法或不在单词集内
            println!("INVALID");
            continue;
        }
        else {//非字母
            let mut flag = true;
            for c in guess.chars() {
                if !c.is_alphabetic() {
                    println!("INVALID");
                    flag = false;
                    break;
                }
            }
            if !flag {
                continue;
            }
        }
        //合法情况
        //guess的颜色集show
        let mut show = ['X';5];
        
        //非困难模式
        if !args.difficult {
            //记录该输入的单词
            if args.stats {
                if let Some(ref mut x) = return_value {
                    let count = x.word_list.entry(guess.clone()).or_insert(0);
                    *count+=1;
                }
            }
            game.guesses.push(guess.to_uppercase().clone());
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if let Some(y) = answer.chars().nth(i) {
                        if x == y {//绿色
                            show[i]='G';
                            green_letters[i] = x;
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                            if let Some(z) = alphabet.get_mut(&x) {
                                *z = 'G';
                            }
                        }
                    }
                }
            }
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if show[i] != 'G' {
                        if answer_map.get(&x) == Some(&0) || answer_map.get(&x) == None  {//红色
                            show[i]='R';
                            if let Some(z) = alphabet.get_mut(&x) {
                                if *z == 'X' {
                                    *z = 'R'; 
                                }
                            }
                        }
                        else {//黄色
                            show[i]='Y';
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                            if let Some(z) = alphabet.get_mut(&x) {
                                if *z != 'G' {
                                    *z = 'Y'; 
                                }
                            }
                        }
                    }
                }
            }
        }
        //困难模式
        else {
            //首先增加判断合法条件
            let mut flag =true;
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if green_letters[i] != '0' && green_letters[i] != x {
                        println!("INVALID");
                        flag = false;
                        break;
                    }
                }
            }
            if flag {
                for i in yellow_letters.clone() {
                    if !guess.contains(i) {
                            println!("INVALID");
                            flag = false;
                            break;
                    }
                }
            }
            if !flag {
                continue;
            }
            if args.stats {
                if let Some(ref mut x) = return_value {
                    let count = x.word_list.entry(guess.clone()).or_insert(0);
                    *count+=1;
                }
            }
            game.guesses.push(guess.to_uppercase().clone());
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if let Some(y) = answer.chars().nth(i) {
                        if x == y {//绿色
                            show[i] = 'G';
                            green_letters[i] = x;
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                            if let Some(z) = alphabet.get_mut(&x) {
                                *z = 'G';
                            }
                        }
                    }
                }
            }
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if show[i] != 'G'  {
                            if answer_map.get(&x) == Some(&0) || answer_map.get(&x) == None {//红色
                            show[i] = 'R';
                            if let Some(z) = alphabet.get_mut(&x) {
                                if *z == 'X' {
                                    *z = 'R'; 
                                }
                            }
                        }
                        else {//黄色
                            show[i] = 'Y';
                            yellow_letters.push(x);
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                            if let Some(z) = alphabet.get_mut(&x) {
                                if *z != 'G' {
                                    *z = 'Y'; 
                                }
                            }
                        }
                    }
                }
            }
        }
        //输出
        for c in show {
            print!("{}",c);
        }
        print!(" ");
        for c in 'a'..'{' {
            if let Some(x) = alphabet.get_mut(&c) {
                print!("{}", x);
            }
        }
        println!("");
        if guess == answer {//猜词正确
            println!("CORRECT {}", chances);
            if args.stats {
                if let Some(ref mut x) = return_value {
                    x.win = true;
                    x.attempt = chances;
                }
            }
            if let Some(ref mut x) = u.games {
                x.push(game);
            }
            let result = to_string_pretty(&u).unwrap();
            if let Some(path) = &args.state{
                fs::write(path, result).unwrap();
            }
            return return_value;
        }
        chances += 1;
    }
    //猜词失败
    println!("FAILED {}", answer.to_uppercase());
    if args.stats {
        if let Some(ref mut x) = return_value {
            x.attempt = chances - 1;
        }
    }
    if let Some(ref mut x) = u.games {
        x.push(game);
    }
    let result = to_string_pretty(&u).unwrap();
    if let Some(path) = &args.state{
        fs::write(path, result).unwrap();
    }
    return return_value;
}

///Run automatically and print average attempts
fn game_round_automatic(args: &Args) {
    println!("Do you want to test the {} by the algorithm of {}? {}/{}",
                console::style("average guess attempts").bold().yellow(),
                console::style("Information Entrophy").bold().green(),
                console::style("[Y]").bold().yellow(),
                console::style("[N]").bold().red()); 
    let command: char = read!();
    if command == 'N' { return; }
    let mut final_words: BTreeSet<String> = BTreeSet::new();
    let mut acceptable_words: BTreeSet<String> = BTreeSet::new();
    let mut entrophy: BinaryHeap<WordEntrophy> = BinaryHeap::new();
    let mut guess_attempt = [0; 7];
    //导入文件词库
    if let Some(path) = &args.finalset {
        final_words = read_to_list(path.clone());
    }
    else {
        for word in FINAL {
            final_words.insert(word.to_string());
        }
    }
    if let Some(path) = &args.acceptableset {
        acceptable_words = read_to_list(path.clone());
    }
    else {
        for word in ACCEPTABLE {
            acceptable_words.insert(word.to_string());
        }
    }

    let mut tot = 0;
    if args.finalset.is_none() && args.acceptableset.is_none() {
        guess_attempt = [111, 0, 38, 473, 818, 615, 260];
        println!("{} : {}/{}, {:.2}{}", 
            console::style("FAILED").bold().red(),
            console::style(guess_attempt[0]).bold().green(),
            console::style(final_words.len()).bold().green(),
            console::style((guess_attempt[0] as f64/ final_words.len() as f64) * 100.0).bold().yellow(),
            console::style("%").bold().yellow(),);
        for i in 1..7 {
            tot += i * guess_attempt[i];
            println!("{} {} : {}/{}, {:.2}{}", 
            console::style(i).bold().red(),
            console::style("attempt(s)").bold().red(),
            console::style(guess_attempt[i]).bold().green(),
            console::style(final_words.len()).bold().green(),
            console::style((guess_attempt[i] as f64/ final_words.len() as f64) * 100.0).bold().yellow(),
            console::style("%").bold().yellow(),);
        }
        println!("{}: {:.4}",console::style("Average attempts").bold().red(),
                        console::style(tot as f64 / final_words.len() as f64).bold().green(),);
        return;
    }
    let mut cnt = 0;
    for answer in final_words.clone() {
        println!("Processing: {}/{}",cnt, final_words.len());
        cnt += 1;
        let mut reasonable_words = acceptable_words.clone();//指示针对猜测是否是合法单词集
        if let Some(path) = &args.acceptableset {
            acceptable_words = read_to_list(path.clone());
            entrophy = information_entrophy(&acceptable_words);
        }
        else {
            for word in ACCEPTABLE {
                acceptable_words.insert(word.to_string());
            }
            let config = fs::read_to_string("src/acceptable.json").unwrap();
            let parsed: Value = serde_json::from_str(&config).unwrap();
            let obj: Map<String, Value> = parsed.as_object().unwrap().clone();
            for i in obj {
                let x: i64 = serde_json::from_value(i.1).unwrap();
                entrophy.push(WordEntrophy(i.0.clone(), x));
            }
        }


    let mut flag = true;
    let mut chances = 1;
    while chances <=6 {
        //新建answer字符集映射
        let mut answer_map: HashMap<char, i32> = HashMap::new();
        for c in answer.chars() {
            if let Some(x) = answer_map.get_mut(&c) {
                *x += 1;
            }
            answer_map.entry(c).or_insert(1);
        }
        //输入guess
        //信息熵提示
        let mut guess = String::new();
        if let Some(word) = entrophy.pop(){
            guess = word.0.clone();
        }
        if guess == answer {//直接猜出答案
            guess_attempt[chances] += 1;
            flag = false;
            break;
        }
        let mut show = ['X'; 5];
        for i in 0..5 {
            if let Some(x) = guess.chars().nth(i) {
                    if let Some(y) = answer.chars().nth(i) {
                        if x == y {
                            show[i] = 'G';
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                        }
                    }
                }
            }
            for i in 0..5 {
                if let Some(x) = guess.chars().nth(i) {
                    if show[i] != 'G' {
                        if answer_map.get(&x) == Some(&0) || answer_map.get(&x) == None {
                            show[i] = 'R';
                        }
                        else {
                            show[i] = 'Y';
                            if let Some(z) = answer_map.get_mut(&x) {
                                *z -= 1;
                            }
                        }
                    }
                }
            } 
        //颜色输出结果
        let mut state = [0; 5];
        for i in 0..5 {
            let color = show[i];
            match color {
                'G' => { state[i] = 2; },
                'Y' => { state[i] = 1; },
                'R' => { state[i] = 0; },
                 _ => unimplemented!()
            }
        }  
        //更新信息熵集
        entrophy.clear();
        let reasonable_words_copy = reasonable_words.clone();
        reasonable_words.clear();
        for word in &reasonable_words_copy {
            let mut flag = true;
            let mut target_map: HashMap<char, i32> = HashMap::new();
            for c in guess.chars() {
                if let Some(x) = target_map.get_mut(&c) {
                    *x += 1;
                }
                target_map.entry(c).or_insert(1);
            }
            let mut map: HashMap<char, i32> = HashMap::new();
            for c in word.chars() {
                if let Some(x) = map.get_mut(&c) {
                    *x += 1;
                }
                map.entry(c).or_insert(1);
            }
            //Green
            for i in 0..5 {
                if state[i] == 2 && word.chars().nth(i) != guess.chars().nth(i) {
                    flag = false;
                    break;
                }
                else if state[i] == 2 && word.chars().nth(i) == guess.chars().nth(i) {
                    if let Some(x) = word.chars().nth(i) {
                        if let Some(z) = target_map.get_mut(&x) {
                            *z -= 1;
                        }
                        if let Some(z) = map.get_mut(&x) {
                            *z -= 1;
                        }
                    }
                }
            }
            if !flag { continue; }
            //Yellow
            for i in 0..5 {
                if state[i] == 1 {
                    if word.chars().nth(i) == guess.chars().nth(i) { flag = false; break; }
                    else if let Some(x) = guess.chars().nth(i) {
                        if let Some(z) = map.get_mut(&x) {
                            if *z == 0 { flag = false; break; }
                            else { *z -= 1; }
                        }
                        else { flag = false; break; }
                    }
                }
            }
            if !flag { continue; }
            //Red
            for i in 0..5 {
                if state[i] == 0 {
                    if let Some(x) = guess.chars().nth(i) {
                        if let Some(z) = map.get_mut(&x) {
                            if *z != 0 { flag = false; break; }
                        }
                    }
                }
            }
            if !flag { continue; }
            if flag {
                reasonable_words.insert(word.clone());
            }
        }
        entrophy = information_entrophy(&reasonable_words);
        chances += 1;       
    }
    if flag { guess_attempt[0] += 1; }
    }
    println!("{} : {}/{}, {:.2}{}", 
            console::style("FAILED").bold().red(),
            console::style(guess_attempt[0]).bold().green(),
            console::style(final_words.len()).bold().green(),
            console::style((guess_attempt[0] as f64/ final_words.len() as f64) * 100.0).bold().yellow(),
            console::style("%").bold().yellow(),);
        for i in 1..7 {
            tot += i * guess_attempt[i];
            println!("{} {} : {}/{}, {:.2}{}", 
            console::style(i).bold().red(),
            console::style("attempt(s)").bold().red(),
            console::style(guess_attempt[i]).bold().green(),
            console::style(final_words.len()).bold().green(),
            console::style((guess_attempt[i] as f64/ final_words.len() as f64) * 100.0).bold().yellow(),
            console::style("%").bold().yellow(),);
        }
    println!("{}: {:.4}",console::style("Average attempts").bold().red(),
                    console::style(tot as f64 / final_words.len() as f64).bold().green(),);
}

//One wordle round
fn game_round(flag: bool, args:&Args) -> Option<GameResult> {
    if let true = flag {
        game_round_normal(&args)
    }
    else {
        game_round_test(&args)
    }
}


/// The main function for the Wordle game
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse();
    if let Some(path) = &args.config {
        let args_config = read_from_file_config(path);
        if let None = args.word { args.word = args_config.word; }
        if let false = args.random { args.random = args_config.random; }
        if let 1 = args.day { args.day = args_config.day; }
        if let 114514 = args.seed { args.seed = args_config.seed; }
        if let false = args.difficult { args.difficult = args_config.difficult; }
        if let false = args.stats { args.stats = args_config.stats; }
        if let None = args.finalset { args.finalset = args_config.finalset; }
        if let None = args.acceptableset { args.acceptableset = args_config.acceptableset; }
        if let None = args.state { args.state = args_config.state; }
    }
    let is_tty = atty::is(atty::Stream::Stdout);
    if is_tty {
        println!(
            "I am in a tty. Please print {}!",
            console::style("colorful characters").bold().blink().blue()
        );
        print!("{}", console::style("Your name: ").bold().red());
        io::stdout().flush().unwrap();
    
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        println!("Welcome to wordle, {}!", line.trim());
    
    }
    if is_tty { game_round_automatic(&args); }
    //记录测试信息的数据
    let mut win_round = 0;
    let mut lose_round = 0;
    let mut tot_attempt = 0;
    let mut average: f64;
    let mut words_dict: HashMap<String, i32> = HashMap::new();
    if let Some(path) = &args.state {//加载前几轮信息
        let u = read_from_file_user(path);
        for x in &u.games {
            for round in x{
                if round.answer == round.guesses[round.guesses.len() - 1] {
                    win_round += 1;
                    tot_attempt += round.guesses.len() as i32;
                }
                else {
                    lose_round += 1;
                }
                for word in &round.guesses {
                    let count = words_dict.entry(word.clone()).or_insert(0);
                    *count += 1;
                }
            }
        }
    }
    loop{
        let gameresult = game_round(is_tty, &args);
        
        if args.stats {//输出测试信息
            if let Some(ref x)=gameresult {
                if x.win {
                    win_round += 1;
                    tot_attempt += x.attempt;
                }
                else{
                    lose_round += 1;
                }
                if win_round == 0 {
                    average = 0.0;
                }
                else {
                    average = ( tot_attempt as f64 ) / ( win_round as f64 );
                }
                if !is_tty {//测试模式按要求输出
                    print!("{} {} {:.2}", win_round, lose_round, average);
                    println!("");
                }
                else {//交互模式
                    println!("Game Statistics:");
                    println!("Win rate: {:.2}", console::style(win_round as f64 / (win_round + lose_round) as f64).bold().green());
                    println!("Average attempts of wins: {:.2}", console::style(average).bold().cyan());
                }
                let mut words_heap: BinaryHeap<WordDict> = BinaryHeap::new();
                for (key, value) in &x.word_list {
                    let count = words_dict.entry(key.to_uppercase().clone()).or_insert(0);
                    *count+=value;
                }
                for (key, value) in &words_dict {
                    words_heap.push(WordDict(key.to_uppercase().clone(),*value));
                }
                let mut i = 0;
                if is_tty {
                    println!("Your preferred words: ");
                }
                while !words_heap.is_empty() && i < 5 {
                    if let Some(x) = words_heap.pop(){
                        if !is_tty {//测试模式
                            print!("{} {}",x.0.to_uppercase(), x.1);
                        }
                        else {//交互模式
                            println!("{}---{} time(s)",
                                    console::style(x.0.to_uppercase()).bold().magenta(),
                                    console::style(x.1).bold().blue())
                        }
                    }
                    i += 1;
                    if !words_heap.is_empty() && i < 5 && !is_tty {
                        print!(" ");
                    }
                }
                if !is_tty { println!(""); }
            } 
        }
        if args.word == None {//没有使用 -w/--word 参数指定答案
            if !is_tty {//测试模式
                let command: char = read!();
                if command == 'N' {
                    break;
                }
                else if command == 'Y' {
                    if args.random {
                        args.day += 1;
                    }  
                }
            }
            else {//交互模式
                print!("Would you like to start another round? {}/{} ",
                        console::style("[Y]").bold().yellow(),
                        console::style("[N]").bold().red());
                let command: char = read!();
                if command == 'N' {
                    println!("{}",console::style("Thanks for playing!").bold().blink().color256(114));
                    break;
                }
                else if command == 'Y' {
                    if args.random {
                        args.day += 1;
                    }  
                }
            }
        }
        else {
            break;
        }
    }
    Ok(())
}

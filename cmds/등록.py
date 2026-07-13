# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        import json
        import uuid

        if line == '':
            ob.sendLine('☞ 사용법: [물품] 등록')
            return

        mob = ob.env.findObjName('진영')
        if mob == None:
            ob.sendLine('☞ 이곳에서는 불가능해요.')
            return

        name, order = getNameOrder(line)
        item = ob.findObjInven(name, order)
        if item == None:
            ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return

        if item['종류'] != '무기':
            ob.sendLine('☞ 등록할 수 없습니다.')
            return
        if item.checkAttr('아이템속성', '줄수없음'):
            ob.sendLine('☞ 등록할 수 없습니다.')
            return

        if item['고유번호'] != None and item['고유번호'] != '':
            ob.sendLine('☞ 등록할 수 없습니다.')
            return

        itm = {}
        itm['이름'] = item['이름']
        itm['고유번호'] = str(uuid.uuid4())
        itm['등록자'] = ob['이름']
        itm['대여가능'] = True
        itm['인덱스'] = item.index 
        itm['attr'] = item.attr

        ob.remove(item)

        try:
            with open("data/config/book.json", "r", encoding="utf-8") as fr:
                data = json.load(fr)
        except:
            data = []
            pass 
        data.append(itm)
        with open("data/config/book.json", "w", encoding="utf-8") as fw:
            json.dump(data, fw, ensure_ascii=False, indent=2)
        ob.sendLine('☞ 등록 되었습니다.')
